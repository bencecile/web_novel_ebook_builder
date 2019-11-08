use kuchiki::{
    *,
    iter::*,
};

#[derive(Debug)]
pub enum TraverseError {
    CssSelectorCompilation(String),
}

pub struct TreeTraverser<T> {
    root_node: NodeRef,
    data: T,
    hooks: Vec< Hook<T> >,
}
impl <T> TreeTraverser<T> {
    pub fn new(root_node: NodeRef, data: T) -> TreeTraverser<T> {
        TreeTraverser {
            root_node,
            data,
            hooks: Vec::new(),
        }
    }

    pub fn traverse(mut self) -> T {
        for element in Elements(self.root_node.descendants()) {
            for hook in self.hooks.iter_mut() {
                hook.match_element(&mut self.data, &element);
            }
        }
        self.data
    }

    pub fn add_hook(mut self, selectors: &str, negative_selectors: Option<&str>,
    traverse_hook: TraverseHook<T>) -> Result<Self, TraverseError> {
        let selectors = Selectors::compile(selectors)
            .map_err(|_| TraverseError::CssSelectorCompilation(selectors.to_string()))?;
        let negative_selectors = {
            if let Some(neg_s) = negative_selectors {
                Some(Selectors::compile(neg_s)
                    .map_err(|_| TraverseError::CssSelectorCompilation(neg_s.to_string()))?)
            } else {
                None
            }
        };
        self.hooks.push(Hook {
            selectors,
            negative_selectors,
            traverse_hook: traverse_hook,
        });
        Ok(self)
    }
}

pub type TraverseHook<T> = fn(data: &mut T, node: &NodeDataRef<ElementData>);
struct Hook<T> {
    selectors: Selectors,
    negative_selectors: Option<Selectors>,
    traverse_hook: TraverseHook<T>,
}
impl <T> Hook<T> {
    fn match_element(&mut self, data: &mut T, element: &NodeDataRef<ElementData>) {
        if let Some(neg_selector) = self.negative_selectors.as_ref() {
            if neg_selector.matches(element) {
                return;
            }
        }
        if self.selectors.matches(element) {
            (self.traverse_hook)(data, element);
        }
    }
}
