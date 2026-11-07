{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*            Copyright (C) 1994 M.Mackey. All rights reserved.              *}
{*                                                                           *}
{*                              Enemy Data                                   *}
{*                                                                           *}
{*****************************************************************************}


program enterenemy;
uses crt,xqvars;

var f:file of enemykindtype;
    i:integer;
    r:real;

begin

  for i:=0 to MaxEnemyKinds do
  with enemykind[i] do
  begin
    speed:=121;speed2:=60;curve:=0;curve2:=0;hits:=1;
    firetype:=0;score:=500;deathsound:=explosn;
    fires:=false;follows:=false;explodes:=false;laysmines:=false;
    curves:=false;shootback:=false;rebounds:=false;zoom:=false;
    tribbles:=false;repulses:=false;fireprob:=0;changedir:=0.006;
    changecurve:=0;follow:=0;maxspeed:=false;
  end;

  with enemykind[0] do  {supercrystal}
  begin
    speed:=301;speed2:=150;score:=0;deathsound:=sxtsmash;
    changedir:=0.002;numframes:=5;framespeed:=70;maxspeed:=true;
  end;
  with enemykind[1] do  {explosion}
  begin
    speed:=0;speed2:=0;score:=0;deathsound:=0;
    changedir:=0;numframes:=5;framespeed:=64;
  end;
  with enemykind[2] do  {grunger}
  begin
    speed:=121;speed2:=60;score:=200;
    changedir:=0.006;numframes:=3;framespeed:=40;
  end;
  with enemykind[3] do  {fastmover}
  begin
    speed:=281;speed2:=140;score:=300;curves:=true;
    curve:=6000;curve2:=3000;{maxspeed:=true;}
    changedir:=0.003;changecurve:=0.02;numframes:=3;framespeed:=56;
  end;
  with enemykind[4] do  {shooter}
  begin
    speed:=101;speed2:=50;firetype:=1;score:=300;
    fires:=true;fireprob:=0.01;changedir:=0.006;
    numframes:=3;framespeed:=60;
  end;
  with enemykind[5] do {hibernator}
  begin
    speed:=201;speed2:=100;score:=500;
    rebounds:=true;changedir:=0.003;numframes:=3;framespeed:=56;
  end;
  with enemykind[6] do {hibernator hibernating}
  begin
    speed:=0;speed2:=0;hits:=300;score:=500;
    fireprob:=0;changedir:=0;numframes:=0;framespeed:=32767;
  end;
  with enemykind[7] do  {miner}
  begin
    speed:=121;speed2:=60;score:=600;
    curves:=true;curve:=4000;curve2:=2000;changecurve:=0.1;
    laysmines:=true;fireprob:=0.008;changedir:=0.006;
    numframes:=3;framespeed:=32;
  end;
  with enemykind[8] do  {meeby}
  begin
    speed:=81;speed2:=40;hits:=5;score:=2000;
    follows:=true;changedir:=0.006;follow:=0.01;
    numframes:=5;framespeed:=28;
  end;
  with enemykind[9] do  {retaliator}
  begin
    speed:=121;speed2:=60;firetype:=3;score:=1000;deathsound:=0;
    shootback:=true;changedir:=0.006;numframes:=3;framespeed:=64;
  end;
  with enemykind[10] do  {terrier}
  begin
    speed:=121;speed2:=60;score:=1000;
    zoom:=true;changedir:=0.02;numframes:=3;framespeed:=120;
  end;
  with enemykind[11] do  {doing}
  begin
    speed:=121;speed2:=60;firetype:=4;score:=1000;
    fires:=true;fireprob:=0.005;changedir:=0.003;
    numframes:=3;framespeed:=128;
  end;
  with enemykind[12] do  {snipe}
  begin
    speed:=131;speed2:=65;firetype:=5;score:=1250;
    fires:=true;fireprob:=0.004;changedir:=0.004;
    numframes:=3;framespeed:=26;
  end;
  with enemykind[13] do {tribble mother ship}
  begin
    speed:=100;speed2:=50;score:=1500;
    tribbles:=true;changedir:=0.01;numframes:=3;framespeed:=32;
  end;
  with enemykind[14] do {tribbles}
  begin
    speed:=220;speed2:=110;score:=500;
    curves:=true;curve:=1000;curve2:=500;changecurve:=0.1;
    changedir:=0.005;numframes:=3;framespeed:=48;
  end;
  with enemykind[15] do  {buckshot}
  begin
    speed:=101;speed2:=50;firetype:=2;score:=1500;
    fires:=true;fireprob:=0.03;changedir:=0.006;
    numframes:=3;framespeed:=36;
  end;
  with enemykind[16] do  {cluster}
  begin
    speed:=81;speed2:=40;firetype:=6;score:=5000;deathsound:=retaliate;
    explodes:=true;changedir:=0.02;numframes:=3;framespeed:=32;
  end;
  with enemykind[17] do  {sticktight}
  begin
    speed:=101;speed2:=50;score:=2000;
    follows:=true;changedir:=0;follow:=1;
    numframes:=3;framespeed:=40;
  end;
  with enemykind[18] do {repulsor}
  begin
    speed:=141;speed2:=70;score:=7500;
    follows:=true;changedir:=0.01;follow:=0.01;repulses:=true;
    numframes:=5;framespeed:=60;
  end;

  assign(f,'xquest.enm');
  rewrite(f);
  for i:=0 to maxenemykinds do write(f,enemykind[i]);
  close(f);
end.
