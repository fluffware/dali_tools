use dali_tools as dali;
//use dali::drivers::driver::{DALIcommandError};
use dali::utils::discover;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::stream::StreamExt;

#[tokio::main]
async fn main() {
    dali::drivers::init().unwrap();
    let driver = dali::drivers::driver::open("default").unwrap();
    let mut found = discover::find_quick(Arc::new(Mutex::new(driver)));
    while let Some(d) = found.next().await {
        println!("{:?}", d);
    }
}
