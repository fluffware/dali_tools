use dali_tools as dali;
//use dali::drivers::driver::{DALIcommandError};
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::utils::discover;
use futures::executor::block_on;
use std::sync::{Arc, Mutex};
use futures::stream::StreamExt;

fn main() {
    let driver = Helvar510driver::new();
    let mut found = discover::find_quick(Arc::new(Mutex::new(driver)));
    block_on(async {
        while let Some(d) = found.next().await {
            println!("{:?}", d);
        }
    });
    
}
