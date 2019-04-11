use std::collections::HashMap;
use std::io::{Read, Write, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, mpsc::{Sender, Receiver, TryRecvError}};
use std::thread;

use uuid::Uuid;

const DEFAULT: &str = "0:0";

type Item = String;

enum Message {
    Create(Uuid),
    Set((Uuid, Item)),
    Remove(Uuid),
    Request(Sender<Vec<Item>>)
}

fn message_bus(receiver: Receiver<Message>) {
    let mut storage: HashMap<Uuid, Item> = HashMap::new();

    loop {
        match receiver.try_recv() {
            Ok(message) => {
                match message {
                    Message::Create(id) => {
                        storage.insert(id, DEFAULT.to_string());
                    }
                    Message::Set((id, value)) => {
                        if let Some(key) = storage.get_mut(&id) {
                            *key = value;
                        }
                    }
                    Message::Remove(id) => {
                        storage.remove(&id);
                    }
                    Message::Request(sender) => {
                        sender.send(storage.values()
                            .map(|x| x.clone()).collect::<Vec<_>>())
                            .expect("Unable to send data upon request.");
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                println!("Spawner has disconnected.");
                break;
            }
            _ => {}
        }
    }
}

fn spawner(listener: TcpListener, sender: Sender<Message>) {
    let mut handles = Vec::new();

    for stream in listener.incoming() {
        let id = Uuid::new_v4();

        println!("Connection `{}` created.", id.to_hyphenated().to_string());

        sender.send(Message::Create(id.clone()))
            .expect("Unable to send creation message to bus.");

        let clone = sender.clone();
        handles.push(thread::spawn(move || {
            connection(stream.unwrap(), clone, id);
        }));
    }

    for handle in handles {
        handle.join()
            .expect("The thread being joined panicked.");
    }
}

fn connection(mut stream: TcpStream, sender: Sender<Message>, id: Uuid) {
    stream.set_nonblocking(true)
        .expect("Unable to make stream non-blocking.");

    let mut buffer: Vec<u8> = Vec::new();
    let (internal_sender, receiver) = mpsc::channel();

    loop {
        sender.send(Message::Request(internal_sender.clone()))
            .expect("Unable to send request to bus.");

        if let Ok(message) = receiver.recv() {
            let mut value = message.iter().fold(String::new(), |mut acc, x| {
                acc.push_str(x);
                acc.push(',');
                acc
            });
            value.push(';');

            if let Err(_) = stream.write_all(&value.into_bytes()) {
                println!("`{}` disconnected.", id.to_hyphenated().to_string());
                sender.send(Message::Remove(id))
                    .expect("Unable to send message for removal.");
                break;
            }
        } else {
            panic!("Request channel disconnected.");
        }

        if let Err(err) = stream.read_to_end(&mut buffer) {
            if let ErrorKind::WouldBlock = err.kind() {} else {
                println!("`{}` disconnected.", id.to_hyphenated().to_string());
                sender.send(Message::Remove(id))
                    .expect("Unable to send message for removal.");
                break;
            }
        }

        let string = String::from_utf8(buffer).unwrap();
        let values = string.split("\n").collect::<Vec<_>>();
        buffer = Vec::new();

        for value in values {
            if value.len() > 0 {
                sender.send(Message::Set((id.clone(), value.to_string())))
                    .expect("Unable to set new position.");
            }
        }
    }
}

fn main() {
            
    let listener = TcpListener::bind("0.0.0.0:63076")
        .expect("Unable to bind listener.");

    let (sender, receiver) = mpsc::channel();

    let handles = vec![
        thread::spawn(|| { message_bus(receiver); }),
        thread::spawn(|| { spawner(listener, sender); })
    ];

    for handle in handles {
        handle.join()
            .expect("The thread being joined panicked.");
    }
}
