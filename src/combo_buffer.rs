use crossterm::event::KeyCode;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
enum NodeInner {
    More(Vec<ComboNode>),
    Combo(Combo),
}

#[derive(Debug, Clone)]
struct ComboNode {
    node: KeyCode,
    inner: NodeInner,
}

#[derive(Clone, Copy, Debug)]
pub enum Combo {
    Delete,
}

macro_rules! n {
    ($c:expr $(,$next:expr)*$(,)?) => {
        ComboNode {
            node: KeyCode::Char($c),
            inner: NodeInner::More(vec![$($next),*]),
        }
    };
    ($c:expr => $combo:ident) => {
        ComboNode {
            node: KeyCode::Char($c),
            inner: NodeInner::Combo(Combo::$combo)
        }
    }
}

lazy_static! {
    static ref COMBO_TREE: Vec<ComboNode> = vec![n!('d', n!('d' => Delete))];
}

#[derive(Debug, Clone)]
pub struct ComboBuffer {
    cursor: &'static [ComboNode],
}

impl Default for ComboBuffer {
    fn default() -> Self {
        Self {
            cursor: COMBO_TREE.as_slice(),
        }
    }
}

impl ComboBuffer {
    pub fn combo(&mut self, k: KeyCode) -> Option<Combo> {
        match self.cursor.iter().find(|n| n.node == k).map(|n| &n.inner) {
            Some(NodeInner::Combo(c)) => {
                self.clear();
                return Some(*c);
            }
            Some(NodeInner::More(more)) => {
                self.cursor = more;
            }
            None => *self = Self::default(),
        }
        None
    }

    pub fn clear(&mut self) {
        *self = Self::default()
    }
}
