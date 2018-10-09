extern crate tokio;
extern crate tokio_timer;
extern crate futures;
extern crate bui_backend;

use std::rc::Rc;
use std::cell::RefCell;

use futures::{Future, Stream};
use bui_backend::change_tracker::ChangeTracker;

#[test]
fn test_change_tracker() {

    #[derive(Clone,PartialEq,Debug)]
    struct StoreType {
        val: i32,
    }

    let data_store_rc = Rc::new(RefCell::new(ChangeTracker::new(StoreType { val: 123 })));
    let rx = data_store_rc.borrow_mut().get_changes();
    let rx_printer = rx.for_each(|(old_value, new_value)| {
                                     assert!(old_value.val == 123);
                                     assert!(new_value.val == 124);
                                     futures::future::err(()) // return error to abort stream
                                 });

    let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();

    let dsclone = data_store_rc.clone();
    // Create a future to cause a change
    let cause_change = tokio_timer::Delay::new(
        std::time::Instant::now())
        .and_then(move |_| {
            {
                let mut data_store = dsclone.borrow_mut();
                data_store.modify(|scoped_store| {
                    assert!((*scoped_store).val == 123);
                    (*scoped_store).val += 1;
                });
            }
            Ok(())
        })
        .map_err(|_| ());

    rt.spawn(cause_change);
    match rt.block_on(rx_printer) {
        Ok(_) => panic!("should not get here"),
        Err(()) => (),
    }

    assert!(data_store_rc.borrow().as_ref().val == 124);
}

#[test]
fn test_dropped_rx() {

    #[derive(Clone,PartialEq,Debug)]
    struct StoreType {
        val: i32,
    }

    let data_store_rc = Rc::new(RefCell::new(ChangeTracker::new(StoreType { val: 123 })));

    {
        let _rx = data_store_rc.borrow_mut().get_changes();
        // drop rx at end of this scope
    }

    let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();

    let dsclone = data_store_rc.clone();
    // Create a future to cause a change
    let cause_change = tokio_timer::Delay::new(
        std::time::Instant::now())
        .and_then(move |_| {
            {
                let mut data_store = dsclone.borrow_mut();
                data_store.modify(|scoped_store| {
                    assert!((*scoped_store).val == 123);
                    (*scoped_store).val += 1;
                });
            }
            Ok(())
        })
        .map_err(|_| ());

    match rt.block_on(cause_change) {
        Ok(_) => (),
        Err(()) => panic!("should not get here"),
    }

    assert!(data_store_rc.borrow().as_ref().val == 124);
}

#[test]
fn test_multiple_changes_no_rx() {

    #[derive(Clone,PartialEq,Debug)]
    struct StoreType {
        val: i32,
    }

    let data_store_rc = Rc::new(RefCell::new(ChangeTracker::new(StoreType { val: 123 })));
    let rx = data_store_rc.borrow_mut().get_changes();
    let _rx_printer = rx.for_each(|(_, _)| -> Result<(),()> {
            panic!("receiver should not be called");
        });

    let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();

    let dsclone1 = data_store_rc.clone();
    let dsclone2 = data_store_rc.clone();

    // Create a future to cause a change
    let cause_change1 = tokio_timer::Delay::new(
        std::time::Instant::now())
        .and_then(move |_| {
            {
                let mut data_store = dsclone1.borrow_mut();
                data_store.modify(|scoped_store| {
                    (*scoped_store).val += 1;
                });
            }
            Ok(())
        })
        .map_err(|_| ());
    rt.spawn(cause_change1);

    // Create a future to cause a change
    let cause_change2 = tokio_timer::Delay::new(
        std::time::Instant::now())
        .and_then(move |_| {
            {
                let mut data_store = dsclone2.borrow_mut();
                data_store.modify(|scoped_store| {
                    (*scoped_store).val += 1;
                });
            }
            Ok(())
        })
        .map_err(|_| ());
    rt.spawn(cause_change2);

    rt.run().unwrap();
    assert!(data_store_rc.borrow().as_ref().val == 125);
}