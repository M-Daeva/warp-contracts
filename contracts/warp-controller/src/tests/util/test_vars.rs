use controller::{
    condition::{Condition, Expr, GenExpr, NumOp, NumValue},
    variable::{QueryExpr, StaticVariable, Variable, VariableKind},
};
use schemars::_serde_json::json;

use crate::util::variable::{all_vector_vars_present, hydrate_vars};

use controller::variable::QueryVariable;
use cosmwasm_std::{testing::mock_env, WasmQuery};
use cosmwasm_std::{to_binary, BankQuery, Binary, ContractResult, OwnedDeps};

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{from_slice, Empty, Querier, QueryRequest, SystemError, SystemResult};
use std::marker::PhantomData;

#[test]
fn test_vars() {
    let test_msg = "{\"execute\":{\"test\":\"$WARPVAR.test\"}}".to_string();

    let _idx = test_msg.find("\"$WARPVAR\"");

    let _new_str = test_msg.replace("\"$WARPVAR.test\"", "\"input\"");
}

#[test]
fn test_all_vector_vars_present() {
    let condition = Condition::Expr(Box::new(Expr::Uint(GenExpr {
        left: NumValue::Env(controller::condition::NumEnvValue::Time),
        op: NumOp::Gt,
        right: NumValue::Ref("$warp.variable.next_execution".to_string()),
    })));

    let cond_string = serde_json_wasm::to_string(&condition).unwrap();

    let var = Variable::Static(StaticVariable {
        kind: VariableKind::Uint,
        name: "next_execution".to_string(),
        value: "1".to_string(),
        update_fn: None,
    });

    assert_eq!(all_vector_vars_present(&vec![var], cond_string), true);
}

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(MockQuerier::new(&[]));

    OwnedDeps {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> SystemResult<ContractResult<Binary>> {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                });
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(
        &self,
        request: &QueryRequest<Empty>,
    ) -> SystemResult<ContractResult<Binary>> {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg: _,
            }) => {
                // Mock logic for the Wasm::Smart case
                // Here for simplicity, we return the contract_addr and msg as is.

                // Mock logic for the Wasm::Smart case
                // Here we return a JSON object with "address" and "msg" fields.
                let response: String = json!({
                    "address": contract_addr,
                    "msg": "Mock message"
                })
                .to_string();

                SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
            }
            QueryRequest::Bank(BankQuery::Balance {
                address: contract_addr,
                denom: _,
            }) => SystemResult::Ok(ContractResult::Ok(
                to_binary(&format!("{}", contract_addr)).unwrap(),
            )),
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier { base }
    }
}

#[test]
fn test_hydrate_vars_nested_variables_binary_json() {
    let deps = mock_dependencies();
    let env = mock_env();

    let var1 = Variable::Query(QueryVariable {
        name: "var1".to_string(),
        kind: VariableKind::Json,
        init_fn: QueryExpr {
            selector: "$".to_string(),
            query: QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract_addr".to_string(),
                msg: Binary::from(r#"{"test":"test"}"#.as_bytes()),
            }),
        },
        value: None,
        reinitialize: false,
        update_fn: None,
    });

    let var2 = Variable::Query(QueryVariable {
        name: "var2".to_string(),
        kind: VariableKind::Json,
        init_fn: QueryExpr {
            selector: "$".to_string(),
            query: QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract_addr".to_string(),
                msg: Binary::from(r#"{"test":"$warp.variable.var1"}"#.as_bytes()),
            }),
        },
        value: None,
        reinitialize: false,
        update_fn: None,
    });

    let vars = vec![var1, var2];
    let hydrated_vars = hydrate_vars(deps.as_ref(), env.clone(), vars, None).unwrap();

    assert_eq!(
        hydrated_vars[1],
        Variable::Query(QueryVariable {
            name: "var2".to_string(),
            kind: VariableKind::Json,
            init_fn: QueryExpr {
                selector: "$".to_string(),
                query: QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "contract_addr".to_string(),
                    msg: Binary::from(
                        r#"{"test":{"address":"contract_addr","msg":"Mock message"}}"#.as_bytes()
                    ),
                }),
            },
            value: Some(r#"{"address":"contract_addr","msg":"Mock message"}"#.to_string()),
            reinitialize: false,
            update_fn: None
        })
    );
}

#[test]
fn test_hydrate_vars_nested_variables_binary() {
    let deps = mock_dependencies();
    let env = mock_env();

    let var1 = Variable::Static(StaticVariable {
        name: "var1".to_string(),
        kind: VariableKind::String,
        value: "static_value".to_string(),
        update_fn: None,
    });

    let init_fn = QueryExpr {
        selector: "$".to_string(),
        query: QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: "$warp.variable.var1".to_string(),
            msg: Binary::from(r#"{"test": "$warp.variable.var1"}"#.as_bytes()),
        }),
    };

    let var2 = Variable::Query(QueryVariable {
        name: "var2".to_string(),
        kind: VariableKind::String,
        init_fn,
        value: None,
        reinitialize: false,
        update_fn: None,
    });

    let vars = vec![var1, var2];
    let hydrated_vars = hydrate_vars(deps.as_ref(), env.clone(), vars, None).unwrap();

    assert_eq!(
        hydrated_vars[1],
        Variable::Query(QueryVariable {
            name: "var2".to_string(),
            kind: VariableKind::String,
            init_fn: QueryExpr {
                selector: "$".to_string(),
                query: QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "static_value".to_string(),
                    msg: Binary::from(r#"{"test": "static_value"}"#.as_bytes()),
                }),
            },
            value: Some(r#"{"address":"static_value","msg":"Mock message"}"#.to_string()),
            reinitialize: false,
            update_fn: None
        })
    );
}
#[test]
fn test_hydrate_vars_nested_variables_non_binary() {
    let deps = mock_dependencies();
    let env = mock_env();

    let var1 = Variable::Static(StaticVariable {
        name: "var1".to_string(),
        kind: VariableKind::String,
        value: "static_value".to_string(),
        update_fn: None,
    });

    let init_fn = QueryExpr {
        selector: "$".to_string(),
        query: QueryRequest::Bank(BankQuery::Balance {
            address: "$warp.variable.var1".to_string(),
            denom: "denom".to_string(),
        }),
    };

    let var2 = Variable::Query(QueryVariable {
        name: "var2".to_string(),
        kind: VariableKind::String,
        init_fn,
        value: None,
        reinitialize: false,
        update_fn: None,
    });

    let vars = vec![var1, var2];
    let hydrated_vars = hydrate_vars(deps.as_ref(), env.clone(), vars, None).unwrap();

    assert_eq!(
        hydrated_vars[1],
        Variable::Query(QueryVariable {
            name: "var2".to_string(),
            kind: VariableKind::String,
            init_fn: QueryExpr {
                selector: "$".to_string(),
                query: QueryRequest::Bank(BankQuery::Balance {
                    address: "static_value".to_string(),
                    denom: "denom".to_string(),
                }),
            },
            value: Some("static_value".to_string()),
            reinitialize: false,
            update_fn: None
        })
    );
}
