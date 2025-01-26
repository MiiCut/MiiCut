use crate::{
    canvas::{CanvasText, Pattern},
    pools::HS,
    prefab::*,
    primitives::primitives::{
        GetPrimitiveState, Primitive, PrimitiveControls, PrimitiveCurve, SetPrimitiveState,
        SetPrimitiveStateFromPos, Vertex, VertexProperty,
    },
    KeysStates, Pointer,
};
use kurbo::{BezPath, Size, Vec2};

/// Example of what a vertex might hold.

/// The polygon stores all vertices and edges in parallel vectors.
#[derive(Debug, Clone)]
pub struct Polygon {
    vertices: Vec<Vertex>,
    primitives: Vec<Primitive>,
    vertices_property: VertexProperty,
}

impl Polygon {
    const GRAB: f64 = 5.;

    /// Start an empty polygon (always with a first vertex)
    pub fn with_first_vertex(pos_first_vertex: Vec2, vertices_property: VertexProperty) -> Self {
        let vertices = vec![Vertex::new(pos_first_vertex, vertices_property)];
        Self {
            vertices,
            primitives: Vec::new(),
            vertices_property,
        }
    }
    /// Add a new vertex at `pos`, connecting it to the previously added vertex
    pub fn add_vertex(&mut self, kind: PrimitiveCurve, property: VertexProperty, pos: Vec2) {
        let new_index = self.vertices.len();
        // Create the new vertex
        self.vertices.push(Vertex::new(pos, property));
        // Connect the previous vertex to the new one.
        self.primitives
            .push(Primitive::new(kind, new_index - 1, new_index));
    }
    pub fn finish(&mut self, kind: PrimitiveCurve) -> bool {
        // We only close the polygon if we have at least 3 vertices
        let n = self.vertices.len();
        if n > 2 {
            self.primitives.push(Primitive::new(kind, n - 1, 0));
            true
        } else {
            false
        }
    }
    /// Example: quickly see the primitives positions for drawing or geometry.
    pub fn primitives_pos(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.primitives.iter().map(move |e| {
            let p1 = self.vertices[e.get_start()].get_pos().pos;
            let p2 = self.vertices[e.get_end()].get_pos().pos;
            (p1, p2)
        })
    }
    /// Change to next curve and return the curve middle position
    pub fn primitive_next_curve(&mut self, primitive: &mut Primitive) -> Vec2 {
        primitive.next_curve();
        let p1 = self.vertices[primitive.get_start()].get_pos().pos;
        let p2 = self.vertices[primitive.get_end()].get_pos().pos;
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => (p1 + p2) / 2.,
            CurveArc => (p1 + p2) / 2.,
        }
    }
    /// Change to prev curve and return the curve middle position
    pub fn primitive_prev_curve(&mut self, primitive: &mut Primitive) -> Vec2 {
        primitive.prev_curve();
        let s_pos = self.vertices[primitive.get_start()].get_pos().pos;
        let e_pos = self.vertices[primitive.get_end()].get_pos().pos;
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => (s_pos + e_pos) / 2.,
            CurveArc => (s_pos + e_pos) / 2.,
        }
    }

    pub fn get_primitive_controls_positions(&self, primitive: &Primitive) -> Vec<Vec2> {
        let s_pos = self.vertices[primitive.get_start()].get_pos().pos;
        let e_pos = self.vertices[primitive.get_end()].get_pos().pos;
        let mut controls = vec![];
        // Only add the start (end is the next primitive start)
        controls.push(s_pos);
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => primitive
                .get_line()
                .get_all_controls_positions(s_pos, e_pos),
            CurveArc => primitive.get_arc().get_all_controls_positions(s_pos, e_pos),
        }
    }

    // Get the primitive current curve paths and patterns
    pub fn get_primitive_paths_and_patterns(
        &self,
        primitive: &Primitive,
        das: &Size,
    ) -> (BezPath, Pattern) {
        let s_pos = self.vertices[primitive.get_start()].get_pos().pos;
        let e_pos = self.vertices[primitive.get_end()].get_pos().pos;
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => primitive
                .get_line()
                .get_paths_and_patterns(s_pos, e_pos, das),
            CurveArc => primitive
                .get_arc()
                .get_paths_and_patterns(s_pos, e_pos, das),
        }
    }
    pub fn get_primitive_controls_paths_and_patterns(
        &self,
        primitive: &Primitive,
        das: &Size,
    ) -> Vec<(BezPath, Pattern)> {
        let s = &self.vertices[primitive.get_start()];
        let e = &self.vertices[primitive.get_end()];

        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];

        // Only add the start (end is the next primitive start)
        paths_patterns.push((
            modifiers_path(s.get_pos().pos, 1., Self::GRAB),
            s.get_pattern(),
        ));

        use PrimitiveCurve::*;
        paths_patterns.extend(match primitive.get_curve() {
            CurveLine => primitive.get_line().get_mod_paths_and_patterns(
                s.get_pos().pos,
                e.get_pos().pos,
                das,
            ),
            CurveArc => primitive.get_arc().get_mod_paths_and_patterns(
                s.get_pos().pos,
                e.get_pos().pos,
                das,
            ),
        });
        paths_patterns
    }
    // Get the primitive current curve dimensions paths and patterns
    pub fn get_primitive_dimensions_paths_and_patterns(
        &self,
        primitive: &Primitive,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let s_pos = self.vertices[primitive.get_start()].get_pos().pos;
        let e_pos = self.vertices[primitive.get_end()].get_pos().pos;
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => primitive
                .get_line()
                .get_dimensions_paths_and_patterns(s_pos, e_pos, das),
            CurveArc => primitive
                .get_arc()
                .get_dimensions_paths_and_patterns(s_pos, e_pos, das),
        }
    }

    pub fn primitive_toogle_curve_property(&mut self, primitive: &mut Primitive) -> Vec2 {
        let s_pos = *self.vertices[primitive.get_start()].get_pos();
        let e_pos = *self.vertices[primitive.get_end()].get_pos();
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => {
                primitive.get_line_mut().toggle();
                primitive
                    .get_line_mut()
                    .update_primitives_vars(s_pos, e_pos)
            }
            CurveArc => {
                primitive.get_arc_mut().toggle();
                primitive.get_arc_mut().update_primitives_vars(s_pos, e_pos)
            }
        }
    }
    pub fn primitive_save_vars(&mut self, primitive: &mut Primitive) {
        self.vertices[primitive.get_start()].save_vars();
        self.vertices[primitive.get_end()].save_vars();
        primitive.get_line_mut().save_vars();
        primitive.get_arc_mut().save_vars();
    }
    pub fn primitive_restore_saved(&mut self, primitive: &mut Primitive) {
        self.vertices[primitive.get_start()].restore_vars();
        self.vertices[primitive.get_end()].restore_vars();
        primitive.get_line_mut().restore_vars();
        primitive.get_arc_mut().restore_vars();
    }
    pub fn update_primitives_vars(&mut self, primitive: &mut Primitive) -> Vec2 {
        let s = *self.vertices[primitive.get_start()].get_pos();
        let e = *self.vertices[primitive.get_end()].get_pos();
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => primitive.get_line_mut().update_primitives_vars(s, e),
            CurveArc => primitive.get_arc_mut().update_primitives_vars(s, e),
        }
    }

    pub fn primitive_get_state(
        &self,
        primitive: &Primitive,
        get: GetPrimitiveState,
    ) -> Option<Vec2> {
        use GetPrimitiveState::*;
        use PrimitiveCurve::*;
        let s = self.vertices[primitive.get_start()].get_pos();
        let e = self.vertices[primitive.get_end()].get_pos();

        match get {
            IsHS(hs) => match primitive.get_curve() {
                CurveLine => primitive.get_line().get_state(s.pos, e.pos, IsHS(hs)),
                CurveArc => primitive.get_arc().get_state(s.pos, e.pos, IsHS(hs)),
            },
            IsStartHS(hs) => match hs {
                HS::Select => s.selected.then_some(s.pos),
                HS::Highlight => s.highlighted.then_some(s.pos),
            },
            IsOtherModifiersHS(hs) => {
                use PrimitiveCurve::*;
                match primitive.get_curve() {
                    CurveLine => {
                        primitive
                            .get_line()
                            .get_state(s.pos, e.pos, IsOtherModifiersHS(hs))
                    }
                    CurveArc => primitive
                        .get_arc()
                        .get_state(s.pos, e.pos, IsOtherModifiersHS(hs)),
                }
            }
        }
    }
    pub fn primitive_set_state(&mut self, primitive: &mut Primitive, set: SetPrimitiveState) {
        use PrimitiveCurve::*;
        use SetPrimitiveState::*;
        let s = self.vertices[primitive.get_start()].get_pos();
        let e = self.vertices[primitive.get_end()].get_pos();
        match set {
            SetHS(hs, value) => match primitive.get_curve_mut() {
                CurveLine => primitive
                    .get_line_mut()
                    .set_state(s.pos, e.pos, SetHS(hs, value)),
                CurveArc => primitive
                    .get_arc_mut()
                    .set_state(s.pos, e.pos, SetHS(hs, value)),
            },
            SetStartHS(hs, value) => match hs {
                HS::Select => self.vertices[primitive.get_start()].get_pos_mut().selected = value,
                HS::Highlight => {
                    self.vertices[primitive.get_start()]
                        .get_pos_mut()
                        .highlighted = value
                }
            },
            SetAllOtherModifiersHS(hs, value) => {
                use PrimitiveCurve::*;
                match primitive.get_curve() {
                    CurveLine => primitive.get_line_mut().set_state(
                        s.pos,
                        e.pos,
                        SetAllOtherModifiersHS(hs, value),
                    ),
                    CurveArc => primitive.get_arc_mut().set_state(
                        s.pos,
                        e.pos,
                        SetAllOtherModifiersHS(hs, value),
                    ),
                }
            }
        }
    }
    pub fn set_state_from_pos(
        &mut self,
        primitive: &mut Primitive,
        pointer: &mut Pointer,
        set: SetPrimitiveStateFromPos,
    ) {
        use PrimitiveCurve::*;
        use SetPrimitiveStateFromPos::*;
        let s = *self.vertices[primitive.get_start()].get_pos();
        let e = *self.vertices[primitive.get_end()].get_pos();
        match set {
            SetHSFromPos(hs) => match primitive.get_curve() {
                CurveLine => primitive.get_line_mut().set_state_from_pos(
                    s.pos,
                    e.pos,
                    pointer,
                    SetHSFromPos(hs),
                ),
                CurveArc => primitive.get_arc_mut().set_state_from_pos(
                    s.pos,
                    e.pos,
                    pointer,
                    SetHSFromPos(hs),
                ),
            },
            SetStartHSFromPos(hs) => {
                let state = (s.pos - pointer.pos()).hypot() < Self::GRAB;
                match hs {
                    HS::Select => {
                        if state {
                            self.vertices[primitive.get_start()].get_pos_mut().selected = true;
                            pointer.set_pos(s.pos);
                            pointer.save_pos();
                        } else {
                            self.vertices[primitive.get_start()].get_pos_mut().selected = false;
                        }
                    }
                    HS::Highlight => {
                        if state {
                            self.vertices[primitive.get_start()]
                                .get_pos_mut()
                                .highlighted = true;
                            pointer.set_pos(s.pos);
                            pointer.save_pos();
                        } else {
                            self.vertices[primitive.get_start()]
                                .get_pos_mut()
                                .highlighted = false;
                        }
                    }
                }
            }
            SetOthersModifiersHSFromPos(hs) => {
                use PrimitiveCurve::*;
                match primitive.get_curve() {
                    CurveLine => primitive.get_line_mut().set_state_from_pos(
                        s.pos,
                        e.pos,
                        pointer,
                        SetOthersModifiersHSFromPos(hs),
                    ),
                    CurveArc => primitive.get_arc_mut().set_state_from_pos(
                        s.pos,
                        e.pos,
                        pointer,
                        SetOthersModifiersHSFromPos(hs),
                    ),
                }
            }
        }
    }
    pub fn move_control_selected(
        &mut self,
        primitive: &mut Primitive,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool {
        let s = *self.vertices[primitive.get_start()].get_pos();
        let e = *self.vertices[primitive.get_end()].get_pos();
        use PrimitiveCurve::*;
        match primitive.get_curve() {
            CurveLine => {
                primitive
                    .get_line_mut()
                    .move_control_selected(s.pos, e.pos, pointer, keys_states)
            }
            CurveArc => {
                primitive
                    .get_arc_mut()
                    .move_control_selected(s.pos, e.pos, pointer, keys_states)
            }
        }
    }
}
