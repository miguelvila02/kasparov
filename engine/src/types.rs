#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
    pub fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceKind {
    Pawn = 0,
    Knight,
    Bishop,
    Rook,
    Queen,
    King
}

impl PieceKind {
    pub fn idx(self) -> usize {
        self as usize
    }
}

// Casas
pub type Square = u8;

pub fn sq(file: u8, rank: u8) -> Square {
    rank * 8 + file
}
pub fn file_of(s:Square) -> u8 {
    s % 8
}
pub fn rank_of(s:Square) -> u8 {
    s / 8
}

pub fn parse_square(s: &str) -> Option<Square> {
    let b = s.as_bytes();
    if b.len() != 2 {
        return None;
    }
    let f = b[0].wrapping_sub(b'a');
    let r = b[1].wrapping_sub(b'1');
    if f < 8 && r < 8{ Some(sq(f, r)) } else { None }
}

pub fn square_name(s: Square) -> String {
    format!("{}{}", (b'a' + file_of(s)) as char, (b'1' + rank_of(s)) as char)
}

// Bitboards
pub type Bitboard = u64;

pub const fn bit(s: Square) -> Bitboard {
    1u64 << s
}

/// Devolve a casa do bit menos significativo e remove-o
pub fn pop_lsb(bb: &mut Bitboard) -> Square {
    let s = bb.trailing_zeros() as Square;
    *bb &= *bb - 1;
    s
}

// Direitos de roque
pub const WK: u8 = 1; //brancas, lado do rei
pub const WQ: u8 = 2; //brancas, lado da rainha
pub const BK: u8 = 4;
pub const BQ: u8 = 8;

// Conversão das peças
pub fn piece_from_char(c: char) -> Option<(Color, PieceKind)> {
    let color = if c.is_ascii_uppercase() { Color::White } else { Color::Black };
    let kind = match c.to_ascii_lowercase() {
        'p' => PieceKind::Pawn,
        'n' => PieceKind::Knight,
        'b' => PieceKind::Bishop,
        'r' => PieceKind::Rook,
        'q' => PieceKind::Queen,
        'k' => PieceKind::King,
        _ => return None,
    };
    Some((color, kind))
}

pub fn piece_to_char(color: Color, kind: PieceKind) -> char {
    let c = match kind {
        PieceKind::Pawn => 'p',
        PieceKind::Knight => 'n',
        PieceKind::Bishop => 'b',
        PieceKind::Rook => 'r',
        PieceKind::Queen => 'q',
        PieceKind::King => 'k',
    };
    if color == Color::White {
        c.to_ascii_uppercase()
    } else {
        c
    }
}