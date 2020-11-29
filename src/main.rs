use dali_tools as dali;
//use dali::drivers::driver::{DALIcommandError};
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::utils::discover;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::stream::StreamExt;

#[tokio::main]
async fn main() {
    let driver = Helvar510driver::new();
    let mut found = discover::find_quick(Arc::new(Mutex::new(Box::new(driver))));
    while let Some(d) = found.next().await {
        println!("{:?}", d);
    }
}
