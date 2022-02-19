use cozy_chess::{Board, Move, Piece, PieceMovesIter, Square};

pub struct MoveOrdering<'a> {
    board: &'a Board,
    stage: MoveOrderingStage,
    hashmove: Option<Move>,
    killer: Move,
    captures: Vec<(Move, i8)>,
    quiets: Vec<PieceMovesIter>,
    underpromotions: Vec<Move>,
}

#[derive(Clone, Copy)]
enum MoveOrderingStage {
    Hashmove,
    PrepareCaptures,
    Captures,
    Quiets,
    Underpromotions,
}

const PIECE_ORDINALS: [i8; Piece::NUM] = [0, 1, 1, 2, 3, 4];

impl<'a> MoveOrdering<'a> {
    pub fn new(board: &'a Board, hashmove: Option<Move>, killer: Move) -> Self {
        MoveOrdering {
            board,
            stage: match hashmove {
                Some(_) => MoveOrderingStage::Hashmove,
                None => MoveOrderingStage::PrepareCaptures,
            },
            hashmove,
            killer: match Some(killer) != hashmove {
                true => killer,
                false => Move {
                    from: Square::A1,
                    to: Square::A1,
                    promotion: None,
                },
            },
            captures: vec![],
            quiets: vec![],
            underpromotions: vec![],
        }
    }

    fn hashmove(&mut self) -> Option<Move> {
        self.stage = MoveOrderingStage::PrepareCaptures;
        self.hashmove
    }

    fn prepare_captures(&mut self) -> Option<Move> {
        self.stage = MoveOrderingStage::Captures;
        let theirs = self.board.colors(!self.board.side_to_move());
        self.board.generate_moves(|mut mvs| {
            if self.killer.from == mvs.from && mvs.to.has(self.killer.to) {
                // Killer is legal; give it a middle rank but in the captures list
                self.captures.push((self.killer, 0));
                if self.killer.promotion.is_none() {
                    // don't accidentally prune underpromotions.
                    // this means we might return the same move twice, but w/e
                    mvs.to &= !self.killer.to.bitboard();
                }
            }

            let mut quiets = mvs;
            quiets.to &= !theirs;
            self.quiets.push(quiets.into_iter());

            mvs.to &= theirs;
            for mv in mvs {
                if Some(mv) == self.hashmove {
                    continue;
                }
                let attacker = PIECE_ORDINALS[mvs.piece as usize];
                let victim = PIECE_ORDINALS[self.board.piece_on(mv.to).unwrap() as usize] * 4;
                if matches!(mv.promotion, None | Some(Piece::Queen)) {
                    self.captures.push((mv, victim - attacker));
                } else {
                    self.underpromotions.push(mv);
                }
            }
            false
        });
        self.captures()
    }

    fn captures(&mut self) -> Option<Move> {
        if self.captures.is_empty() {
            self.stage = MoveOrderingStage::Quiets;
            return self.quiets();
        }

        let mut index = 0;
        for i in 1..self.captures.len() {
            if self.captures[i].1 > self.captures[index].1 {
                index = i;
            }
        }

        Some(self.captures.swap_remove(index).0)
    }

    fn quiets(&mut self) -> Option<Move> {
        loop {
            let iter = match self.quiets.last_mut() {
                Some(iter) => iter,
                None => {
                    self.stage = MoveOrderingStage::Underpromotions;
                    return self.underpromotions();
                }
            };

            let mv = match iter.next() {
                Some(mv) => mv,
                None => {
                    self.quiets.pop();
                    continue;
                }
            };

            if Some(mv) == self.hashmove {
                continue;
            }

            if matches!(mv.promotion, None | Some(Piece::Queen)) {
                return Some(mv);
            } else {
                self.underpromotions.push(mv);
                continue;
            }
        }
    }

    fn underpromotions(&mut self) -> Option<Move> {
        self.underpromotions.pop()
    }
}

impl Iterator for MoveOrdering<'_> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stage {
            MoveOrderingStage::Hashmove => self.hashmove(),
            MoveOrderingStage::PrepareCaptures => self.prepare_captures(),
            MoveOrderingStage::Captures => self.captures(),
            MoveOrderingStage::Quiets => self.quiets(),
            MoveOrderingStage::Underpromotions => self.underpromotions(),
        }
    }
}
