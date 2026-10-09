pub struct Zobrist {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub castling: [u64; 16], // um valor para cada combinação dos 4 bits
    pub ep_file: [u64; 8],
    pub side: u64,           // aplicado quando jogam as pretas
}

// xorshift64*: devolve (novo_estado, valor)
const fn next(state: u64) -> (u64, u64) {
    let mut x = state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    (x, x.wrapping_mul(0x2545_F491_4F6C_DD1D))
}

const fn build() -> Zobrist {
    let mut z = Zobrist {
        pieces: [[[0; 64]; 6]; 2],
        castling: [0; 16],
        ep_file: [0; 8],
        side: 0,
    };
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;

    let mut c = 0;
    while c < 2 {
        let mut p = 0;
        while p < 6 {
            let mut q = 0;
            while q < 64 {
                let (ns, v) = next(s);
                s = ns;
                z.pieces[c][p][q] = v;
                q += 1;
            }
            p += 1;
        }
        c += 1;
    }
    let mut i = 0;
    while i < 16 {
        let (ns, v) = next(s);
        s = ns;
        z.castling[i] = v;
        i += 1;
    }
    let mut f = 0;
    while f < 8 {
        let (ns, v) = next(s);
        s = ns;
        z.ep_file[f] = v;
        f += 1;
    }
    let (_, v) = next(s);
    z.side = v;
    z
}

pub static ZOBRIST: Zobrist = build();