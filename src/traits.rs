use crate::canvas::{CanvasText, Pattern};
use kurbo::{BezPath, Size, Vec2};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GetEntityState {
    IsSelected,
    IsHighlighted,
    IsAnyModifierSelected,
    IsAnyModifierHighlighted,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetEntityState {
    SetSelect(bool),
    SelectFromPos(Vec2, f64, f64),
    SetHighlight(bool),
    HighlightFromPos(Vec2, f64, f64),

    SelectAllModifiers(bool),
    SelectModifierFromPos(Vec2, f64, f64),

    HighlightAllModifiers(bool),
    HighlightModifierFromPos(Vec2, f64, f64),
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
    fn restore_saved(&mut self);
    fn get_vars(&self) -> Self::Kindvars;
    fn set_vars(&mut self, vars: &Self::Kindvars);
    fn good_size(&self) -> bool;

    fn get_state(&self, get: GetEntityState) -> Option<Vec2>;
    fn set_state(&mut self, set: SetEntityState);

    fn toggle_prop(&mut self);

    fn move_position(&mut self, dpos: Vec2, snap: f64) -> Option<Vec2>;
    fn move_modifier(
        &mut self,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2>;
    fn get_position(&self) -> Vec2;

    fn get_paths(&self, drawing_area_size: &Size) -> Vec<BezPath>;
    fn get_paths_and_patterns(&self, canvas_size: &Size) -> Vec<(BezPath, Pattern)>;
    fn get_modifiers_paths(&self, drawing_area_size: &Size) -> Vec<(BezPath, Pattern)>;
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>);
    fn get_pattern_modifiers(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
}
