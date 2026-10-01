//! A SQL editor's results: the grid of the last statement that returned
//! rows, and the messages of every statement.

use egui::Ui;

use crate::app::App;
use crate::model::{ConnTabId, TabId};

pub fn show(_app: &mut App, _ui: &mut Ui, _tab: ConnTabId, _id: TabId) {}
