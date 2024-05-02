mod gql;

use worker::*;

#[event(fetch)]
async fn main(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let variables = gql::get_workers_analytics_query::Variables {
        account_tag: Some("".to_string()),
        datetime_start: Some("".to_string()),
        datetime_end: Some("".to_string()),
        script_name: Some("".to_string()),
    };
    let _ = gql::perform_my_query(variables);
    Response::ok("Hello, World!")
}
