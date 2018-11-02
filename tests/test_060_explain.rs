extern crate flexi_logger;
extern crate hdbconnect;
#[macro_use]
extern crate log;
extern crate bigdecimal;
extern crate num;

extern crate serde_json;

#[allow(unused_imports)]
use flexi_logger::{LogSpecification, Logger, ReconfigurationHandle};
use hdbconnect::{ConnectParams, Connection, HdbResult};

// cargo test test_060_explain -- --nocapture
#[test]
pub fn test_060_explain() -> HdbResult<()> {
    let mut log_handle = Logger::with_env_or_str("info, test_060_explain = info")
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let mut connection = get_authenticated_connection()?;
    run(&mut log_handle, &mut connection)?;
    info!("{} calls to DB were executed", connection.get_call_count()?);
    Ok(())
}

fn run(_log_handle: &mut ReconfigurationHandle, connection: &mut Connection) -> HdbResult<()> {
    info!("use EXPLAIN and verify it works");

    let result =
        connection.dml("DELETE FROM explain_plan_table WHERE statement_name = 'test_explain'")?;
    debug!("cleanup (deletion result = {:?})", result);

    let count: usize = connection
        .query("select count(*) from EXPLAIN_PLAN_TABLE")?
        .try_into()?;
    assert_eq!(count, 0);

    debug!("create the plan");
    connection
        .exec("EXPLAIN PLAN SET STATEMENT_NAME = 'test_explain' FOR select 'FOO' from dummy")?;

    let count: u32 = connection
        .query("select count(*) from EXPLAIN_PLAN_TABLE")?
        .try_into()?;
    debug!("read the plan size (no of lines = {})", count);
    assert!(count > 0);

    let result: Vec<(String, String)> = connection
        .query(
            "SELECT Operator_Name, Operator_ID \
             FROM explain_plan_table \
             WHERE statement_name = 'test_explain';",
        )?.try_into()?;
    debug!("obtain the plan: {:?}", result);

    Ok(())
}

fn get_authenticated_connection() -> HdbResult<Connection> {
    let params = get_std_connect_params()?;
    trace!("params: {:?}", params);
    Connection::new(params)
}

fn get_std_connect_params() -> HdbResult<ConnectParams> {
    //let version = "2_0";
    let version = "2_3";
    let path = format!("./.private/db_{}_std.url", version);
    ConnectParams::from_file(path)
}