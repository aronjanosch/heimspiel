use std::fs;

use heimspiel_core::progress::{Message, Progress, Window};

/// A window program that writes what it receives to `received`.
fn window(received: &std::path::Path) -> Window {
    Window::new(vec![
        "sh".into(),
        "-c".into(),
        format!("cat > '{}'", received.display()),
    ])
}

fn messages(received: &std::path::Path) -> Vec<Message> {
    fs::read_to_string(received)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn sends_each_step_to_the_window_and_waits_for_it_to_close() {
    let tmp = tempfile::tempdir().unwrap();
    let received = tmp.path().join("received");
    let cover = tmp.path().join("7.png");
    let window = window(&received);

    window.start("Tekken 3", Some(&cover), "Downloading");
    window.advance(1, 4);
    // Too soon after the last update to be sent, unless it is the last one.
    window.advance(2, 4);
    window.advance(4, 4);
    window.finish();

    assert_eq!(
        messages(&received),
        [
            Message::Start {
                game: "Tekken 3".into(),
                cover: Some(cover),
                action: "Downloading".into(),
            },
            Message::Advance {
                done_bytes: 1,
                total_bytes: 4,
            },
            Message::Advance {
                done_bytes: 4,
                total_bytes: 4,
            },
        ]
    );
}

#[test]
fn opens_the_window_for_a_failure_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let received = tmp.path().join("received");

    window(&received).fail("could not download Tekken 3");

    assert_eq!(
        messages(&received),
        [Message::Fail {
            message: "could not download Tekken 3".into()
        }]
    );
}

#[test]
fn opens_no_window_for_steps_that_were_never_started() {
    let tmp = tempfile::tempdir().unwrap();
    let received = tmp.path().join("received");
    let window = window(&received);

    window.advance(1, 4);
    window.finish();

    assert!(!received.exists());
}
