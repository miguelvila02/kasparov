#!/usr/bin/env bash
# Plays kasparov-new against kasparov-base until SPRT decides.
~/chess-bins/fastchess \
  -engine cmd=$HOME/chess-bins/kasparov-new name=new \
  -engine cmd=$HOME/chess-bins/kasparov-base name=base \
  -each tc=8+0.08 proto=uci \
  -openings file=$HOME/chess-bins/UHO_Lichess_4852_v1.epd format=epd order=random \
  -rounds 2000 -repeat -concurrency 3 \
  -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05