use super::Position;
use crate::types::*;

#[derive(Debug, PartialEq)]
pub enum FenError {
    WrongFieldCount,
    BadPlacement,
    BadSide,
    BadCastling,
    BadEnPassant,
    BadCounter,
}

impl Position {
    pub fn from_fen(fen: &str) -> Result<Position, FenError> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        // Aceita FEN com 4 campos (sem contadores) ou 6
        if parts.len() != 4 && parts.len() != 6 {
            return Err(FenError::WrongFieldCount);
        }
        let mut pos = Position::empty();

        // 1. Peças (da 8.ª fila para a 1.ª)
        let rows: Vec<&str> = parts[0].split('/').collect();
        if rows.len() != 8 {
            return Err(FenError::BadPlacement);
        }
        for (i, row) in rows.iter().enumerate() {
            let rank = 7 - i as u8;
            let mut file = 0u8;
            for ch in row.chars() {
                if let Some(d) = ch.to_digit(10) {
                    if d == 0 || d > 8 {
                        return Err(FenError::BadPlacement);
                    }
                    file += d as u8;
                } else {
                    let (c, p) = piece_from_char(ch).ok_or(FenError::BadPlacement)?;
                    if file >= 8 {
                        return Err(FenError::BadPlacement);
                    }
                    pos.put(c, p, sq(file, rank));
                    file += 1;
                }
                if file > 8 {
                    return Err(FenError::BadPlacement);
                }
            }
            if file != 8 {
                return Err(FenError::BadPlacement);
            }
        }
        
        let kings = PieceKind::King.idx();
        if pos.pieces[0][kings].count_ones() != 1 || pos.pieces[1][kings].count_ones() != 1 {
            return Err(FenError::BadPlacement);
        }

        // 2. Lado a jogar
        pos.side = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::BadSide),
        };

        // 3. Roque
        if parts[2] != "-" {
            for ch in parts[2].chars() {
                pos.castling |= match ch {
                    'K' => WK,
                    'Q' => WQ,
                    'k' => BK,
                    'q' => BQ,
                    _ => return Err(FenError::BadCastling),
                };
            }
        }

        // 4. En passant (só pode estar na 3.ª ou na 6.ª fila)
        if parts[3] != "-" {
            let e = parse_square(parts[3]).ok_or(FenError::BadEnPassant)?;
            if rank_of(e) != 2 && rank_of(e) != 5 {
                return Err(FenError::BadEnPassant);
            }
            pos.ep = Some(e);
        }

        // 5-6. Contadores
        if parts.len() == 6 {
            pos.halfmove = parts[4].parse().map_err(|_| FenError::BadCounter)?;
            pos.fullmove = parts[5].parse().map_err(|_| FenError::BadCounter)?;
        }

        pos.hash = pos.compute_hash();
        Ok(pos)
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for rank in (0..8u8).rev() {
            let mut empty = 0u32;
            for file in 0..8u8 {
                match self.board[sq(file, rank) as usize] {
                    None => empty += 1,
                    Some((c, p)) => {
                        if empty > 0 {
                            s.push(char::from_digit(empty, 10).unwrap());
                            empty = 0;
                        }
                        s.push(piece_to_char(c, p));
                    }
                }
            }
            if empty > 0 {
                s.push(char::from_digit(empty, 10).unwrap());
            }
            if rank > 0 {
                s.push('/');
            }
        }

        s.push_str(if self.side == Color::White { " w " } else { " b " });

        if self.castling == 0 {
            s.push('-');
        } else {
            if self.castling & WK != 0 { s.push('K'); }
            if self.castling & WQ != 0 { s.push('Q'); }
            if self.castling & BK != 0 { s.push('k'); }
            if self.castling & BQ != 0 { s.push('q'); }
        }

        match self.ep {
            Some(e) => s.push_str(&format!(" {}", square_name(e))),
            None => s.push_str(" -"),
        }
        s.push_str(&format!(" {} {}", self.halfmove, self.fullmove));
        s
    }
}