use crate::current_node::FocusedNodeInfo;
use dioxus::{html::geometry::euclid::Point2D, prelude::*};
use dioxus_free_icons::Icon;
use floneum_plugin::plugins::main::types::ValueType;
use floneum_plugin::PluginInstance;
use floneumite::Category;
use petgraph::{graph::NodeIndex, stable_graph::DefaultIx};
use serde::{Deserialize, Serialize};

use crate::edge::{Connection, ConnectionType};
use crate::input::Input;
use crate::node_value::{NodeInput, NodeOutput};
use crate::output::Output;
use crate::{theme, use_application_state, Colored};
use crate::{Point, VisualGraph};

const SNAP_DISTANCE: f32 = 15.;
pub const NODE_KNOB_SIZE: f64 = 5.;
pub const NODE_MARGIN: f64 = 2.;

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub instance: PluginInstance,
    #[serde(skip)]
    pub running: bool,
    #[serde(skip)]
    pub queued: bool,
    #[serde(skip)]
    pub error: Option<String>,
    pub id: NodeIndex<DefaultIx>,
    pub position: Point,
    pub inputs: Vec<Signal<NodeInput>>,
    pub outputs: Vec<Signal<NodeOutput>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Node {
    fn input_connections(&self) -> impl Iterator<Item = Connection> + '_ {
        (0..self.inputs.len())
            .filter_map(|index| {
                let input = self.inputs[index].read();
                if let ValueType::Single(_) = input.definition.ty {
                    Some(Connection {
                        index,
                        ty: ConnectionType::Single,
                    })
                } else {
                    None
                }
            })
            .chain((0..self.inputs.len()).flat_map(|index| {
                let input = self.inputs[index].read();
                let indexes = if let ValueType::Many(_) = input.definition.ty {
                    0..input.value.len()
                } else {
                    0..0
                };
                indexes.map(move |inner| Connection {
                    index,
                    ty: ConnectionType::Element(inner),
                })
            }))
    }

    fn input_count(&self) -> usize {
        let mut inputs = self.inputs.len();
        for input_idx in 0..self.inputs.len() {
            if let Some(ValueType::Many(_)) = self.input_type(Connection {
                index: input_idx,
                ty: ConnectionType::Single,
            }) {
                inputs += self.inputs[input_idx].read().value.len();
            }
        }
        inputs
    }

    pub fn input_type(&self, index: Connection) -> Option<ValueType> {
        match index.ty {
            ConnectionType::Single => self
                .inputs
                .get(index.index)
                .map(|input| input.read().definition.ty),
            ConnectionType::Element(_) => self.element_input_type(index.index),
        }
    }

    pub fn element_input_type(&self, index: usize) -> Option<ValueType> {
        self.inputs
            .get(index)
            .and_then(|input| match &input.read().definition.ty {
                ValueType::Many(ty) => Some(ValueType::Many(*ty)),
                ValueType::Single(_) => None,
            })
    }

    pub fn input_color(&self, index: Connection) -> String {
        match self.input_type(index) {
            Some(ty) => ty.color(),
            None => "black".to_string(),
        }
    }

    pub fn input_is_list(&self, index: Connection) -> bool {
        matches!(self.input_type(index), Some(ValueType::Many(_)))
    }

    pub fn output_type(&self, index: usize) -> Option<ValueType> {
        self.outputs
            .get(index)
            .map(|input| input.read().definition.ty)
    }

    pub fn output_is_list(&self, index: usize) -> bool {
        matches!(self.output_type(index), Some(ValueType::Many(_)))
    }

    pub fn output_color(&self, index: usize) -> String {
        match self.output_type(index) {
            Some(ty) => ty.color(),
            None => "black".to_string(),
        }
    }

    pub fn help_text(&self) -> String {
        self.instance.metadata().description.to_string()
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct NodeProps {
    node: Signal<Node>,
}

pub fn Node(props: NodeProps) -> Element {
    let mut application = use_application_state();
    let node = props.node;
    let current_node = node.read();
    let pos = current_node.position;

    rsx! {
        // center UI/Configuration
        div {
            position: "absolute",
            left: "{pos.x}px",
            top: "{pos.y}px",
            onmousedown: move |evt| {
                let mut graph: VisualGraph = consume_context();
                graph.start_dragging_node(&evt, props.node);
            },
            onmousemove: |evt| {
                let mut  graph: VisualGraph = consume_context();
                graph.update_mouse(&evt);
            },
            onmouseup: move |_| {
                let mut graph: VisualGraph = consume_context();
                graph.clear_dragging();

                // Focus or unfocus this node
                let mut application = application.write();
                match &application.currently_focused {
                    Some(currently_focused_node) if currently_focused_node.node == props.node => {
                        application.currently_focused = None;
                    }
                    _ => {
                        application.currently_focused = Some(FocusedNodeInfo{
                            node: props.node,
                            active_example_index: None,
                        } );
                    }
                }
            },

            CenterNodeUI {
                node: props.node,
            }
        }

        // inputs
        for index in 0..current_node.inputs.len() {
            Input {
                node: props.node,
                index,
            }
        }

        // outputs
        for index in 0..current_node.outputs.len() {
            Output {
                node: props.node,
                index,
            }
        }
    }
}

fn CenterNodeUI(props: NodeProps) -> Element {
    let mut application = use_application_state();
    let focused = application.read().currently_focused.map(|n| n.node) == Some(props.node);
    let mut node = props.node;
    {
        if node.with(|n| n.queued) {
            node.with_mut(|node| node.queued = false);
            let application = application.write();
            application.graph.run_node(node);
        }
    }
    let current_node = node.read();
    let name = &current_node.instance.metadata().name;
    let focused_class = if focused {
        "border-2 border-blue-500"
    } else {
        "border"
    };
    let category = match current_node.instance.source().meta() {
        Some(meta) => meta.category,
        None => Category::Other,
    };
    let color = theme::category_bg_color(category);

    rsx! {
        div {
            style: "-webkit-user-select: none; -ms-user-select: none; user-select: none;",
            class: "shadow-sm resize w-32 h-32 flex flex-col rounded-md {focused_class}",
            div {
                class: "flex w-full h-8 flex-shrink-0 items-center justify-center {color} rounded-t-md text-sm font-medium text-black",
                h1 {
                    class: "text-md",
                    "{name}"
                }
            }
            div {
                class: "flex flex-col justify-center items-center",
                button {
                    class: "p-2 border top-0 right-0",
                    onclick: move |_| {
                        application.write().remove(node.read().id)
                    },
                    Icon {
                        width: 15,
                        height: 15,
                        icon: dioxus_free_icons::icons::io_icons::IoTrashOutline,
                    }
                }
                if current_node.running {
                    "Loading..."
                } else {
                    button {
                        class: "p-1 border rounded-md ",
                        onclick: move |_| {
                            node.write().queued = true;
                        },
                        "Run"
                    }
                }
                div { color: "red",
                    if let Some(error) = &current_node.error {
                        p { "{error}" }
                    }
                }
            }
        }
    }
}
