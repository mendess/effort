use std::iter::Peekable;

use crossterm::event::KeyCode;
use lazy_static::lazy_static;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
struct ComboNode {
    key: KeyCode,
    next: ActionOrMore,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
enum ActionOrMore {
    More(Vec<ComboNode>),
    Combo(ComboAction),
}

fn build_combos<const N: usize>(sequences: [(&str, ComboAction); N]) -> Vec<ComboNode> {
    fn build<I: Iterator<Item = char>>(
        mut chars: Peekable<I>,
        tree: &mut Vec<ComboNode>,
        action: ComboAction,
    ) {
        if let Some(key) = chars.next().map(KeyCode::Char) {
            match tree.iter_mut().find(|n| n.key == key) {
                Some(node) => match &mut node.next {
                    ActionOrMore::Combo(_) => panic!("ambiguous combo defined"),
                    ActionOrMore::More(next) => build(chars, next, action),
                },
                None => {
                    let next = if chars.peek().is_some() {
                        let mut more = vec![];
                        build(chars, &mut more, action);
                        ActionOrMore::More(more)
                    } else {
                        ActionOrMore::Combo(action)
                    };
                    tree.push(ComboNode { key, next });
                }
            }
        }
    }
    let mut root = vec![];
    for (sequence, action) in sequences {
        build(sequence.chars().peekable(), &mut root, action)
    }
    root
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComboAction {
    Delete,
    SelectFirst,
}

lazy_static! {
    static ref COMBO_TREE: Vec<ComboNode> = build_combos([
        ("dd", ComboAction::Delete),
        ("gg", ComboAction::SelectFirst)
    ]);
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
    pub fn combo(&mut self, k: KeyCode) -> Option<ComboAction> {
        fn find(s: &[ComboNode], k: KeyCode) -> Option<&ActionOrMore> {
            s.iter().find(|n| n.key == k).map(|n| &n.next)
        }

        match find(self.cursor, k) {
            Some(ActionOrMore::Combo(c)) => {
                self.reset();
                return Some(*c);
            }
            Some(ActionOrMore::More(more)) => {
                self.cursor = more;
            }
            None => match find(&COMBO_TREE, k) {
                Some(ActionOrMore::Combo(c)) => {
                    self.reset();
                    return Some(*c);
                }
                Some(ActionOrMore::More(more)) => {
                    self.cursor = more;
                }
                None => self.reset(),
            },
        }
        None
    }

    pub fn reset(&mut self) {
        *self = Self::default()
    }
}

#[cfg(test)]
mod build_combo_test {
    use crossterm::event::KeyCode;

    use super::*;

    #[test]
    fn one_combo() {
        let expected = vec![ComboNode {
            key: KeyCode::Char('d'),
            next: ActionOrMore::More(vec![ComboNode {
                key: KeyCode::Char('d'),
                next: ActionOrMore::Combo(ComboAction::Delete),
            }]),
        }];

        assert_eq!(expected, build_combos([("dd", ComboAction::Delete)]))
    }

    #[test]
    fn two_independent_combos() {
        let expected = vec![
            ComboNode {
                key: KeyCode::Char('d'),
                next: ActionOrMore::More(vec![ComboNode {
                    key: KeyCode::Char('d'),
                    next: ActionOrMore::Combo(ComboAction::Delete),
                }]),
            },
            ComboNode {
                key: KeyCode::Char('g'),
                next: ActionOrMore::More(vec![ComboNode {
                    key: KeyCode::Char('g'),
                    next: ActionOrMore::Combo(ComboAction::SelectFirst),
                }]),
            },
        ];

        assert_eq!(
            expected,
            build_combos([
                ("dd", ComboAction::Delete),
                ("gg", ComboAction::SelectFirst)
            ])
        )
    }

    #[test]
    fn combos_with_common_start() {
        let expected = vec![
            ComboNode {
                key: KeyCode::Char('d'),
                next: ActionOrMore::More(vec![
                    ComboNode {
                        key: KeyCode::Char('d'),
                        next: ActionOrMore::Combo(ComboAction::Delete),
                    },
                    ComboNode {
                        key: KeyCode::Char('i'),
                        next: ActionOrMore::Combo(ComboAction::SelectFirst),
                    },
                ]),
            },
            ComboNode {
                key: KeyCode::Char('g'),
                next: ActionOrMore::More(vec![ComboNode {
                    key: KeyCode::Char('g'),
                    next: ActionOrMore::Combo(ComboAction::SelectFirst),
                }]),
            },
        ];

        assert_eq!(
            expected,
            build_combos([
                ("dd", ComboAction::Delete),
                ("di", ComboAction::SelectFirst),
                ("gg", ComboAction::SelectFirst)
            ])
        )
    }

    #[test]
    #[should_panic]
    fn ambiguous_combos() {
        build_combos([("dd", ComboAction::Delete), ("ddi", ComboAction::Delete)]);
    }
}

#[cfg(test)]
mod test {
    use super::ComboAction::*;
    use super::*;

    #[test]
    fn dd() {
        let mut buf = ComboBuffer::default();
        assert_eq!(buf.combo(KeyCode::Char('d')), None);
        assert_eq!(buf.combo(KeyCode::Char('d')), Some(Delete));
    }

    #[test]
    fn gg() {
        let mut buf = ComboBuffer::default();
        assert_eq!(buf.combo(KeyCode::Char('g')), None);
        assert_eq!(buf.combo(KeyCode::Char('g')), Some(SelectFirst));
    }

    #[test]
    fn invalid() {
        let mut buf = ComboBuffer::default();
        assert_eq!(buf.combo(KeyCode::Char('d')), None);
        assert_eq!(buf.combo(KeyCode::Char('g')), None);
    }

    #[test]
    fn invalid_then_valid() {
        let mut buf = ComboBuffer::default();
        assert_eq!(buf.combo(KeyCode::Char('d')), None);
        assert_eq!(buf.combo(KeyCode::Char('g')), None);
        assert_eq!(buf.combo(KeyCode::Char('g')), Some(SelectFirst));
    }

    #[test]
    fn combo_cleared_after_hit() {
        let mut buf = ComboBuffer::default();
        assert_eq!(buf.combo(KeyCode::Char('g')), None);
        assert_eq!(buf.combo(KeyCode::Char('g')), Some(SelectFirst));
        assert_eq!(buf.combo(KeyCode::Char('g')), None);
    }

    #[test]
    fn stack_overflow() {
        let mut buf = ComboBuffer::default();
        assert_eq!(buf.combo(KeyCode::Char('j')), None);
        assert_eq!(buf.combo(KeyCode::Char('j')), None);
        assert_eq!(buf.combo(KeyCode::Char('j')), None);
    }
}
