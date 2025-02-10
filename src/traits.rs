use crate::{
    canvas::{CanvasText, Pattern},
    pools::HS,
    KeysStates, Pointer,
};
use kurbo::{BezPath, Rect, Size, Vec2};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GetEntityState {
    IsHS(HS),
    GetFirstControlHS(HS),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetEntityState {
    SetHS(HS, bool),
    SetAllControlsHS(HS, bool),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetEntityStateFromPos {
    SetHSFromPos(HS),
    SetControlHSFromPos(HS),
}

pub trait NewId {
    fn new() -> Self;
}
pub trait ObjectOps {
    type Id: NewId;
    type Kind;

    fn get_id(&self) -> Self::Id;
    fn get_kind(&self) -> &Self::Kind;
    fn get_kind_mut(&mut self) -> &mut Self::Kind;
    fn set_new_id(&mut self, id: Self::Id);
}
pub trait CommonPool {
    fn duplicate<T: ObjectOps + Clone>(&mut self, draw_objs: Vec<T>) -> Vec<T>;
}

pub trait ObjectsFuncs: Debug + Clone {
    const TOLERANCE: f64;
    const GRAB_RADIUS: f64;
    type Kindvars;

    fn save_vars(&mut self);
    fn restore_vars(&mut self);
    fn get_vars(&self) -> Self::Kindvars;
    fn set_vars(&mut self, vars: &Self::Kindvars);
    fn get_state(&self, get: GetEntityState) -> Option<Vec2>;
    fn set_state(&mut self, set: SetEntityState);
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    );
    fn toggle_selected_prop(&mut self);
    fn move_position(&mut self, pointer: &mut Pointer, keys_states: KeysStates) -> bool;
    fn move_controls(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool;
    fn get_position(&self) -> Vec2;
    fn get_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)>;
    fn get_controls_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)>;
    fn get_dimensions_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)>;
}
