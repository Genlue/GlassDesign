//! Light-DOM projection for layout surfaces. Host nodes are never cloned.

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Element, HtmlElement, Node};

use crate::props::error;

#[derive(Clone, Copy)]
enum Slot {
    Body,
    Start,
    End,
}

struct Child {
    node: Node,
    slot: Slot,
    projected_into: Option<Element>,
}

#[derive(Default)]
pub(crate) struct Slots(Vec<Child>);

impl Slots {
    pub fn collect(kind: &str, host: &HtmlElement) -> Result<Self, JsValue> {
        if !matches!(
            kind,
            "card" | "status-bar" | "toolbar" | "capsule" | "glass-panel"
        ) {
            return Ok(Self::default());
        }
        let mut children = Vec::new();
        let mut next = host.first_child();
        while let Some(node) = next {
            next = node.next_sibling();
            let name = node
                .dyn_ref::<Element>()
                .and_then(|el| el.get_attribute("data-glass-slot"));
            let slot = match (kind, name.as_deref()) {
                ("card" | "toolbar" | "capsule" | "glass-panel", None | Some("body" | "")) => {
                    Slot::Body
                }
                ("status-bar", None | Some("start" | "")) => Slot::Start,
                ("status-bar", Some("end")) => Slot::End,
                _ => return Err(error(format!("Invalid {kind} data-glass-slot: {name:?}"))),
            };
            children.push(Child {
                node,
                slot,
                projected_into: None,
            });
        }
        Ok(Self(children))
    }

    pub fn has_end(&self) -> bool {
        self.0.iter().any(|child| matches!(child.slot, Slot::End))
    }

    pub fn project(&mut self, host: &HtmlElement) -> Result<(), JsValue> {
        if self.0.is_empty() {
            return Ok(());
        }
        let root = host
            .last_element_child()
            .ok_or_else(|| error("GlassDesign slot root missing"))?;
        let start = root.query_selector(".c-statusbar__items-start")?;
        let end = root.query_selector(".c-statusbar__items-end")?;
        for child in &mut self.0 {
            let destination = match child.slot {
                Slot::Body => Some(&root),
                Slot::Start => start.as_ref(),
                Slot::End => end.as_ref(),
            }
            .ok_or_else(|| error("GlassDesign slot destination missing"))?;
            destination.append_child(&child.node)?;
            child.projected_into = Some(destination.clone());
        }
        Ok(())
    }

    pub fn restore(&mut self, host: &HtmlElement) {
        for child in &mut self.0 {
            if let Some(destination) = child.projected_into.take() {
                if destination.is_same_node(child.node.parent_node().as_ref()) {
                    let _ = host.append_child(&child.node);
                }
            }
        }
    }
}
