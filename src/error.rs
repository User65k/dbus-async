use crate::command::Command;
use dbus_message_parser::{Interface, Message, ObjectPath};
use futures::channel::mpsc::TrySendError;
use futures::channel::oneshot::Canceled;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

#[derive(Debug)]
pub enum DBusError {
    SendMessage(Message),
    AddMethodCall(ObjectPath),
    DeleteMethodCall(Option<ObjectPath>),
    ListMethodCall(ObjectPath),
    AddMethodCallInterface(Interface),
    DeleteMethodCallInterface(Option<Interface>),
    AddSignal(ObjectPath),
    DeleteSignal,
    ReceiveMessage(Option<Message>),
    Close,
}

impl From<TrySendError<Command>> for DBusError {
    fn from(e: TrySendError<Command>) -> Self {
        match e.into_inner() {
            Command::SendMessage(msg) => DBusError::SendMessage(msg),
            Command::SendMessageOneshot(msg, _) => DBusError::SendMessage(msg),
            Command::SendMessageMpcs(msg, _, _) => DBusError::SendMessage(msg),
            Command::AddMethodCall(object_path, _) => DBusError::AddMethodCall(object_path),
            Command::DeleteMethodCall(object_path) => {
                DBusError::DeleteMethodCall(Some(object_path))
            }
            Command::DeleteMethodCallSender(_) => DBusError::DeleteMethodCall(None),
            Command::DeleteMethodCallReceiver(_) => DBusError::DeleteMethodCall(None),
            Command::ListMethodCall(object_path, _) => DBusError::ListMethodCall(object_path),
            Command::AddMethodCallInterface(object_path, _) => {
                DBusError::AddMethodCallInterface(object_path)
            }
            Command::DeleteMethodCallInterface(interface) => {
                DBusError::DeleteMethodCallInterface(Some(interface))
            }
            Command::DeleteMethodCallInterfaceSender(_) => {
                DBusError::DeleteMethodCallInterface(None)
            }
            Command::DeleteMethodCallInterfaceReceiver(_) => {
                DBusError::DeleteMethodCallInterface(None)
            }
            Command::AddSignal(object_path, _, _) => DBusError::AddSignal(object_path),
            Command::DeleteSignalSender(_) => DBusError::DeleteSignal,
            Command::DeleteSignalReceiver(_) => DBusError::DeleteSignal,
            Command::ReceiveMessage(msg) => DBusError::ReceiveMessage(Some(msg)),
            Command::Close => DBusError::Close,
        }
    }
}

impl From<Canceled> for DBusError {
    fn from(_: Canceled) -> Self {
        DBusError::ReceiveMessage(None)
    }
}

impl From<DBusError> for IoError {
    fn from(e: DBusError) -> Self {
        IoError::new(IoErrorKind::Other, format!("call_hello: {:?}", e))
    }
}

impl Display for DBusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DBusError::SendMessage(msg) => write!(f, "Could not send message: {:?}", msg),
            DBusError::AddMethodCall(object_path) => {
                write!(f, "Could not add channel for method call: {}", object_path)
            }
            DBusError::DeleteMethodCall(object_path) => {
                write!(f, "Could not delete channel for method call")?;
                if let Some(object_path) = object_path {
                    write!(f, ": {}", object_path)
                } else {
                    Ok(())
                }
            }
            DBusError::ListMethodCall(path) => write!(f, "Could not list method call: {}", path),
            DBusError::AddMethodCallInterface(interface) => write!(
                f,
                "Could not add chanell for method call(interface): {}",
                interface
            ),
            DBusError::DeleteMethodCallInterface(interface) => {
                write!(f, "Could not delete channel for method call(interface)")?;
                if let Some(interface) = interface {
                    write!(f, ": {}", interface)
                } else {
                    Ok(())
                }
            }
            DBusError::AddSignal(path) => write!(f, "Could not add channel for signals: {}", path),
            DBusError::DeleteSignal => write!(f, "Could not delete channel for signals"),
            DBusError::ReceiveMessage(msg) => {
                write!(f, "Could not receive response for message: {:?}", msg)
            }
            DBusError::Close => write!(f, "Could not close DBus"),
        }
    }
}

pub type DBusResult<T> = std::result::Result<T, DBusError>;
