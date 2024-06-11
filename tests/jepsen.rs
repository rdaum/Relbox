// Copyright (C) 2024 Ryan Daum <ryan.daum@gmail.com>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

#[path = "./test-support.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};
    use std::rc::Rc;
    use std::sync::Arc;
    use tracing_test::traced_test;

    use relbox::RelBox;
    use relbox::{RelationId, Transaction};

    use crate::support::{History, Type, Value};
    use daumtils::SliceRef;

    use super::*;

    fn from_val(value: i64) -> SliceRef {
        SliceRef::from_bytes(&value.to_le_bytes()[..])
    }
    fn to_val(value: SliceRef) -> i64 {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(value.as_slice());
        i64::from_le_bytes(bytes)
    }

    fn check_expected(
        process: i64,
        _db: Arc<RelBox>,
        tx: &Transaction,
        relation: RelationId,
        expected_values: &Option<Vec<i64>>,
        action_type: Type,
    ) {
        // Expect to read these values from the relation in a scan.
        let tuples = tx.relation(relation).predicate_scan(&|_| true).unwrap();

        let got = tuples
            .iter()
            .map(|t| to_val(t.domain().clone()))
            .collect::<BTreeSet<_>>();

        if let Some(values) = expected_values {
            let expected = values.iter().cloned().collect::<BTreeSet<_>>();

            assert!(
                expected.iter().all(|v| got.contains(v)),
                "T{} at {}, r {} expected {:?} but got {:?}",
                process,
                action_type.as_keyword(),
                relation.0,
                values,
                got
            );
        }
    }

    fn check_completion(
        process: i64,
        db: Arc<RelBox>,
        tx: &Transaction,
        values: Vec<Value>,
        action_type: Type,
    ) {
        for ev in values {
            match ev {
                Value::append(_, register, expect_val) => {
                    let relation = RelationId(register as usize);

                    // The value mentioned should have been added to the relation successfully
                    // (at invoke)
                    let t = tx
                        .relation(relation)
                        .seek_unique_by_domain(from_val(expect_val))
                        .unwrap();
                    let val = to_val(t.domain());
                    assert_eq!(
                        val,
                        expect_val,
                        "T{} at {}, expected {} to be {} after its insert",
                        process,
                        action_type.as_keyword(),
                        register,
                        expect_val
                    );
                }
                Value::r(_, register, expected_values) => {
                    let relation = RelationId(register as usize);

                    // Expect to read these values from the relation in a scan.
                    check_expected(
                        process,
                        db.clone(),
                        tx,
                        relation,
                        &expected_values,
                        action_type,
                    );
                }
            }
        }
    }

    #[traced_test]
    #[test]
    fn test_generate() {
        let tmpdir = tempfile::tempdir().unwrap();

        let db = support::test_db(tmpdir.path().into());

        let lines = include_str!("append-dataset.json")
            .lines()
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>();
        let events = lines
            .iter()
            .map(|l| serde_json::from_str::<History>(l).unwrap());
        let mut processes = HashMap::new();
        for e in events {
            match e.r#type {
                Type::invoke => {
                    // Start a transaction.
                    let tx = Rc::new(db.clone().start_tx());
                    let existing = processes.insert(e.process, tx.clone());
                    assert!(
                        existing.is_none(),
                        "T{} already exists uncommitted",
                        e.process
                    );
                    // Execute the actions
                    for ev in &e.value {
                        match ev {
                            Value::append(_, register, value) => {
                                // Insert the value into the relation.
                                let relation = RelationId(*register as usize);
                                tx.clone()
                                    .relation(relation)
                                    .insert_tuple(from_val(*value), from_val(*value))
                                    .unwrap();
                            }
                            Value::r(_, register, values) => {
                                let relation = RelationId(*register as usize);

                                check_expected(
                                    e.process,
                                    db.clone(),
                                    &tx,
                                    relation,
                                    values,
                                    e.r#type,
                                );
                            }
                        }
                    }
                }
                Type::ok => {
                    // Commit the transaction, expecting the values to be in the relation.
                    let tx = processes.remove(&e.process).unwrap();
                    check_completion(e.process, db.clone(), &tx, e.value, e.r#type);
                    tx.commit().unwrap();
                }
                Type::fail => {
                    // Rollback the transaction.
                    let tx = processes.remove(&e.process).unwrap();
                    check_completion(e.process, db.clone(), &tx, e.value, e.r#type);
                    tx.rollback().unwrap();
                }
            }
        }
    }
}
