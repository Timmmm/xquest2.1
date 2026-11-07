{$UNDEF DEBUGDEMO}
{$UNDEF BONUSLEVEL}
{$UNDEF ZTIMER}
{$UNDEF RECORDCHECK}
{$UNDEF TESTMEM}
{$UNDEF DEBUG}
{$UNDEF CHEATS}
{$UNDEF GUS}
{$UNDEF TESTINIT}
{$UNDEF GODMODE}
{$UNDEF TESTCOLLIDE}
{$UNDEF BACKGROUND}


{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*                (C) 1994 M.Mackey. All rights reserved.                    *}
{*                                                                           *}
{*                             Main Program                                  *}
{*                                                                           *}
{*****************************************************************************}
{$M 8192,0,655360}
program xquest;

uses dos,xlib,mouse,xqinit,xqvars,filexist,keyboard,
     sbunit,starunit,joystick {$IFDEF GUS} ,ultradrv {$ENDIF};
(*********************************************************************
BUGS/ITEMS TO FIX:
X 1) Make joystick code work off PIT 2, not 0
X 2) Fix installation program etc.
X 3) Test sound under high DMA and IRQ numbers
X 4) Option to swap mouse buttons?
X 5) Scrolling on ATI cards & flicker. Fix this.
X 6) Two player mode (alternating)
X 7) Joy/mouse disabled depending on input control option
8) GUS support (native and/or SBOS)
9) Backgrounds?
X 10) Hibernators killed by heavyfire
X 11) Demos need to save random seed, game mode and difficulty level
X 12) Text Font needed (do in DPII ??)
X 13) Level counter keeps going (open-ended)
X 14) SB16 crash on program exit (sound?)
15) Make sure demo playback is stable for long games
X 16) Make really high levels (XQuest God+) stable
X 17) Graphics: mines, attractor, smartbombs. Ship?
X 18) Name: anonymous on null string entry for high scores

Memory:
-draw objects in black rather than store their backgrounds (twice...)
X-do we need both backgr and oldbackgr for enemy? These take lots.
X-maxenemies reduced from 40 to 30
Y-dynamic allocation on enemies instead?
X-store startpic as a cbitmap in memory (NOT smaller: 25k vs 14k)
Y-only load startpic when needed: don't keep in memory.

IDEAS
Increase enemy aggressiveness at higher levels (speed, follow)
Increase #smarts ???
Manual- explain newman scores
Bigger/more random explosions
Mouse cursor on front menu
Gate shrinks/expands
X Gravity well in later missions (hills and valleys)
X Gate-freeze powerup
X Ass-fire powerup
X Directed Fire powerup
Powerup message
Palette animation for mines and crystals
X Better ship sprites
X Better smartbomb sprites
X Better exit gate sprites
Background music

ENEMIES
PacMan :)
X Repulsor enemies
Laser enemies
Crystal-layer enemy
Invisible enemy
Shot-dodging enemy (teleporter)
X "Moth" enemies that move in arcs
Self-destructing clusters
X Enemy that, when shot, is simply deflected away.

SOUNDS

Need sound for - repulsors repulsing    [X]
	       - hibernators being hit  [X]


  OTHER FURTHER WORK:
  Fix RapidFire etc. to work off clock in case frame rate slows Xdont
    - Fast missile writes with total of 4 outs?
	(ie all missiles with x mod 4=0 then x mod 4=1 etc.)
  Only show enemies & missiles actually visible on-screen
     - sorted enemy and missile lists
X  Remove Emissiles on collision (for Shield).
  Fix top gate. Metallic gate opens? Fits sound better.
  Challenge levels?
	       - Stream of type (new) which move fast in straight lines:
		 must shoot.
	       - Lots of crystals to collect in v. short time span?
	       - Do something like BombJack: collect crystals in certain order for bonus!
	       - anything else?
**********************************************************************)
{$IFDEF DEBUG}
var j,k:integer;
    a,ir,d:word;
{$ENDIF}
{$IFDEF DEBUGDEMO}
var debugfile:text;
{$ENDIF}

var i:integer;

{$IFDEF ZTIMER}
const numtimes:word=0;
type  timerrec=record
		 t:word;
		 s:string[40];
	       end;
var   times:array[1..100] of timerrec;

{$L PZTIMER.OBJ}
procedure ZTimerOn;external;
procedure ZTimerOff;external;
function ZTimerReport:word;external;

procedure TimerReport(s:string);
begin
  inc(numtimes);
  times[numtimes].t:=ZTimerReport;
  times[numtimes].s:=s;
end;

procedure PrintTimes;
var i:integer;
    total:longint;
begin
  XTextMode;
  writeln('Timer times: ',numtimes);
  writeln;
  total:=0;
  for i:=1 to numtimes do
  with times[i] do
  begin
    writeln(s,' ',t*0.8381:6:1,' us');
    total:=total+t;
  end;
  writeln;
  writeln('Total time: ',total*0.8381:6:1,' us');
  writeln;
  writeln('Enemies: ',numenemies,' Enemy Missiles: ',numenemymissiles);
  writeln('Ship Missiles: ',ship.nummissiles,' Emines: ',NumEnemyMines,' objects: ',GameInfo[Player].NumObjects,' ',ch);
  readkey;
  halt;
end;

{$ENDIF}


procedure SwapObjects;
var temp:ObjPosType;
begin
  temp:=objects.pos;
  objects.pos:=SavedObjects;
  SavedObjects:=temp;
end;

procedure SetupNewLevel(player:playertype);
var i,j:integer;

  function NoOverlap(Numobj:integer):boolean;
  var i,j:integer;
  begin
    NoOverlap:=true;
    for i:=1 to (NumObj-1) do
    for j:=(i+1) to NumObj do
    with objects do
      if (abs(pos[i].x-pos[j].x)<(width+2)) and (abs(pos[i].y-pos[j].y)<(height+2)) then NoOverlap:=false;
    with objects do
    with pos[NumObj] do
    begin
      if ((xbr+8)>=ShipStartX) and ((ybr+8)>=ShipStartY) and
	(x<=(ShipStartX+ship.width+8)) and (y<=(ShipStartY+ship.height+8))
	  then NoOverlap:=false;           {leave an 8-pixel border minimum around ship start}

      with GameInfo[Player] do
      if GameClocked>0 then
      if ((xbr+8)>=AttractorX) and ((ybr+8)>=AttractorY) and
	(x<=(AttractorX+13+8)) and (y<=(AttractorY+13+8))
	  then NoOverlap:=false;           {leave an 8-pixel border minimum around attractor}

      if (y<(PageHeight div 2 + 7)) and (ybr>(PageHeight div 2 -7))
      and ((x<ShipMinX+15) or (xbr>ShipMaxX-15)) then NoOverlap:=false;
	   {checks for collision with enemy inlets}

      if (levels[GameInfo[Player].level].GateMove=0) and (y<ShipMinY+18)
	and (xbr>Gate[Left].X) and (x<Gate[Right].x) then NoOverlap:=false;
	   {ensure gate is clear}
    end;
  end;  {NoOverlap}

begin  {SetUpNewLevel}
{$IFDEF DEBUG}
  writeln(debugfile, 'Level ',GameInfo[player].level,' r:',randseed);
{$ENDIF}
{$IFDEF DEBUGDEMO}
  writeln(debugfile, 'Level ',GameInfo[player].level,' started. Randseed:',randseed);
{$ENDIF}
{$IFDEF TESTMEM}
      writeln(memfile,'---Start of Level ',GameInfo[Player].level,'. Memavail: ',memavail,' Maxavail: ',maxavail);
{$ENDIF}
  with GameInfo[Player] do
  begin
    if totallevel<=maxlevel then
      NewManScore:=LastNewManScore+levels[level].NewMan
    else
      NewManScore:=LastNewManScore+levels[maxlevel].NewMan;
    NumMines:=levels[level].nummine;
    NumCrystals:=levels[level].numcryst;
    NumSmarts:=0;
    GateMoveCount:=0;
    TimeOnLevel:=0;
    Gate[Left].X:=(PageWidth-levels[level].GateWidth) div 2-Gate[Left].Width;
    Gate[Right].X:=(PageWidth+levels[level].GateWidth) div 2;
    while (random<levels[level].smartprob) and (NumSmarts<levels[level].maxsmart) do
      inc(NumSmarts);
    NumObjects:=NumMines+NumCrystals+NumSmarts;
    NoMines:=(NumMines=0);
    repeat
      AttractorX:=random(PageWidth-120)+60;
      AttractorY:=random(PageHeight-120)+60;
    until abs(AttractorX-ShipStartX)+abs(AttractorY-ShipStartY)>60;
      {Setup attractor position randomly, not near ship start, though}

    for i:=1 to NumObjects do
    begin
      with objects do
      begin
	repeat
	  begin
	    pos[i].x:=random(MineXMax-MineXMin)+MineXMin;
	    pos[i].y:=random(MineYMax-MineYMin)+MineYMin;
	    pos[i].xbr:=pos[i].x+width-1;
	    pos[i].ybr:=pos[i].y+height-1;
	  end
	until NoOverlap(i);  {places new object in a clear region}
	if i<=NumCrystals then pos[i].typ:=crys
	  else if i<=(NumMines+NumCrystals) then pos[i].typ:=mine
	else pos[i].typ:=smart;
	pos[i].delete:=false;
{$IFDEF BONUSLEVEL}
	pos[i].bonus:=false;
{$ENDIF}
      end;
    end;
{$IFDEF BONUSLEVEL}
    if levels[level].bonuslevel then
    with objects.pos[1] do
      bonus:=true;
    BonusLevelCount:=0;
{$ENDIF}
    with PlayerInfo[Player] do
    GameSpeed:=round(BaseGameSpeed* (DiffInfo[DiffLevel].speedfactor+GameClockedSpeedUp[GameClocked]));
  end;
end;   {SetUpNewLevel}

procedure ShowLives;
var s:string[3];
begin
  with GameInfo[Player] do
  begin
    str((lives-1):2,s);
    XText(185,5,true,SplitScrnOffs,'  '+s);
    XPutPBM(185,6,SplitScrnOffs,ShipPic^);
  end;
end;  {ShowLives}

procedure ShowSmartBombs;
var s:string[3];
begin
  with GameInfo[Player] do
  begin
    str((NumSmartBombs):2,s);
    XText(230,5,true,SplitScrnOffs,'  '+s);
    XPutPBM(230,6,SplitScrnOffs,SmartPic^);
  end;
end;  {ShowSmartBombs}

procedure ShowCrystals;
var s:string[3];
begin
  str((GameInfo[Player].NumCrystals):2,s);
  XText(275,5,true,SplitScrnOffs,'  '+s);
  XPutPBM(275,6,SplitScrnOffs,CrystalPic^);
end;  {ShowSmartBombs}

procedure ShowPowerups;
var PU:PowerUpType;
    i:integer;
begin
  i:=85;
  for PU:=Shield to Bounce do
  with PowerUp[PU] do
  begin
    if (value>0) and (i<=157) then
    begin
      position:=i;
      XPutPBM(i,5,SplitScrnOffs,Pic^);
      inc(i,18);
    end else position:=0;
  end;
  if i<=157 then XRectFill(i,5,157+17,20,SplitScrnOffs,14)
end;


Procedure BonusShip;
begin
  with  GameInfo[Player] do
  begin
    LastNewManScore:=NewManScore;
    if totallevel<=maxlevel then
      NewManScore:=LastNewManScore+levels[level].NewMan
    else
      NewManScore:=LastNewManScore+levels[maxlevel].NewMan;
	{calculate score for next new ship}
    inc(Lives);
    ShowLives;
    PlaySound(woohoo);
  end;
end; {NewMan}

Procedure AddToScore(i:longint);
var s:string[20];
    j:integer;
begin
  with GameInfo[Player] do
  begin
    inc(Score,i);
    while Score>=NewManScore do BonusShip;
    str(score:8,s);
{    str(demoptr:8,s);}
    Xtext(10,5,true,SplitScrnOffs,s);
  end;
end;  {AddToScore}

procedure StartNewLevel;
var i,j:integer;
begin
  NumEnemies:=0;                {initialise variables for this level}
  NumExplosions:=0;
  NumEnemyMissiles:=0;
  NumEnemyMines:=0;
  emines.adding:=false;
  LevelFinished:=False;
  ShipDestroyed:=false;
  EnemyEnteringLeft:=0;
  EnemyEnteringRight:=0;
  with GameInfo[Player] do GateMoving:=levels[level].GateMove<>0;

  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page0Offs,0);
  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page1Offs,0);
	    {clear page}

  VisiblePageX:=(ScrnLogicalPixelWidth-ScrnPhysicalPixelWidth) div 2;
  VisiblePageY:=(ScrnLogicalHeight-SplitScrnScanLine) div 2;
  XPageFlip(VisiblePageX,VisiblePageY);
	    {center viewing window on top centre of screen}


  for i:=1 to 5 do  {draw screen border}
  begin
    XLine(10,10-i,Gate[Left].X,10-i,Page0Offs,BColor[i]);
    XLine(Gate[Right].X+Gate[Right].Width,10-i,PageWidth-10,10-i,Page0Offs,BColor[i]);
    XLine(10,PageHeight-5-i,PageWidth-10,PageHeight-5-i,Page0Offs,Bcolor[i]);
    XLine(4+i,10,4+i,PageHeight-9,Page0Offs,bcolor[i]);
    XLine(PageWidth-11+i,10,PageWidth-11+i,PageHeight-9,Page0Offs,bcolor[i]);
  end;

  XPutPBM(Gate[Left].X,0,Page0Offs,Gate[Left].pic^);
  XPutPBM(Gate[Right].X,0,Page0Offs,Gate[Right].pic^);
  if GameInfo[Player].NumCrystals>0 then
    XLine(Gate[Left].X+Gate[Left].Width,7,Gate[Right].X-1,7,Page0Offs,54);
  XPutPBM(0,0,Page0Offs,TLCorner^);
  XPutPBM(PageWidth-10,0,Page0Offs,TRCorner^);
  XPutPBM(PageWidth-10,PageHeight-10,Page0Offs,BRCorner^);
  XPutPBM(0,PageHeight-10,Page0Offs,BLCorner^);
  XPutPBM(0,PageHeight div 2-10,Page0Offs,LEnemyGate[0]^);
  XPutPBM(PageWidth-20,PageHeight div 2-10,Page0Offs,REnemyGate[0]^);
  with GameInfo[Player] do
  if GameClocked>0 then
    XPutPBM(AttractorX-6,AttractorY-6,Page0Offs,Attractor^);

  for i:=1 to 100 do XPutPix(random(ShipMaxX-ShipMinX-2)+ShipMinX+1,
	 {stars}             random(ShipMaxY-ShipMinY-5)+ShipMinY+5,
			     Page0Offs,random(20)+10);
  for i:=1 to 400 do XPutPix(random(ShipMaxX-ShipMinX-2)+ShipMinX+1,
			     random(ShipMaxY-ShipMinY-5)+ShipMinY+5,
			     Page0Offs,random(15)+5);
{$IFDEF BACKGROUND}
  for i:=1 to PageWidth-10 do
  for j:=1 to PageHeight-10 do
    XPutPix(i,j,Page0Offs,(i+j) mod 255);
{$ENDIF}

  for i:=GameInfo[Player].numobjects downto 1 do   {delete any objects hit last run}
  if objects.pos[i].delete then
  begin
    objects.pos[i]:=objects.pos[GameInfo[Player].NumObjects];
    dec(GameInfo[Player].NumObjects);
  end;
  with ship do          {initialise ship}
  begin
    x:=ShipStartX;y:=ShipStartY;
    xbr:=x+width-1;
    ybr:=y+height-1;
    oldx:=x;oldy:=y;
    sx:=x shl 6;sy:=y shl 6;
    delx:=0;dely:=0;
    ShipDir:=0;
    nummissiles:=0;
    XGetPBM(x,y,bmwidth,height,Page0Offs,backgr^);
    XGetPBM(x,y,bmwidth,height,Page0Offs,oldbackgr^);
  end;
  for i:=1 to GameInfo[Player].NumObjects do   {draw (remaining) objects}
  with objects do
  begin
    XGetPBM(pos[i].x,pos[i].y,bmwidth,height,Page0Offs,pos[i].backgr^);
    XPutCBitmap(pos[i].x,pos[i].y,Page0Offs,pic[pos[i].typ]^);
{$IFDEF BONUSLEVEL}
    if pos[i].bonus then with pos[i] do
      XPutCBitmap(x,y,Page0Offs,BCrystalPic^);
{$ENDIF}
  end;

  XCpVidRect(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,0,0,
	     Page0Offs,Page1Offs,ScrnLogicalPixelWidth,ScrnLogicalPixelWidth);
	{copy page 0 to page 1}

  XWindow(0,0,319,24,2,SplitScrnOffs); {initialise status bar}
  AddToScore(0);                       {display score}
  ShowLives;
  ShowSmartBombs;
  ShowCrystals;
  with TimeRecord do                    {initialise countdown timer}
  begin
    X:=TimerX+VisiblePageX;
    Y:=TimerY+VisiblePageY;
    oldX:=X;oldY:=Y;
    with smallfont do
      XGetPBM(X,Y,bmwidth*4,height,Page0Offs,backgr^);
    with smallfont do
      XGetPBM(X,Y,bmwidth*4,height,Page0Offs,oldbackgr^);
  end;
  ShowPowerUps;

  mmotion(i,i);
  i:=0;
  mbutpress(i,i,i,i);   {reset mouse motion and button press counters}
  i:=1;
  mbutpress(i,i,i,i);
end;



procedure GiveBonus;
var TimeTaken,Bonus,i:longint;
    s,s2:string[40];
begin
  with GameInfo[Player] do
  begin
    XPageFlip(VisiblePageX,VisiblePageY);
    with ship do XPutPBM(oldx,oldy,VisiblePageOffs,backgr^);
	      {erase ship}
    TimeTaken:=TimeOnLevel div FrameRate;
{$IFDEF DEBUGDEMO}
    writeln(debugfile,'Level ',totallevel,' finished. Time: ',TimeOnLevel,' ticks = ',TimeTaken);
{$ENDIF}
    XWindow(50+VisiblePageX,40+VisiblePageY,270+VisiblePageX,160+VisiblePageY,2,VisiblePageOffs);
    str(totallevel,s);
    XText(TextWindowX+42,TextWindowY+15,true,VisiblePageOffs,'LEVEL '+s+' COMPLETED');
    str(TimeTaken:3,s);
    XText(TextWindowX+20,TextWindowY+40,true,VisiblePageOffs,'TIME TAKEN: '+s+' SECONDS');
    str((levels[level].Time):3,s);
    XText(TextWindowX+20,TextWindowY+65,true,VisiblePageOffs,'PAR       : '+s+' SECONDS');
    Bonus:=(levels[level].Time-TimeTaken)*500;
    if Bonus<0 then Bonus:=0;
    XText(TextWindowX+20,TextWindowY+90,true,VisiblePageOffs,'BONUS     :  0         ');
    if not DelayorEvent(Ticks div 2) then
    for i:=1 to (bonus div 500) do
    begin
      str(i*500,s);
      XText(TextWindowX+124,TextWindowY+90,false,VisiblePageOffs,s);
      str(timeTaken+i:3,s);
      XText(TextWindowX+116,TextWindowY+40,false,VisiblePageOffs,s);
      if (i mod 3)=1 then PlaySound(countdown);
      if DelayOrEvent(Ticks div 12) then if i<>bonus div 500 then
					i:=(bonus div 500)-1;
    end
    else

    begin
      str(bonus,s);
      XText(TextWindowX+124,TextWindowY+90,false,VisiblePageOffs,s);
      str((timeTaken+bonus div 500):3,s);
      XText(TextWindowX+116,TextWindowY+40,false,VisiblePageOffs,s);
      if bonus>0 then PlaySound(countdown);
    end;
    if bonus>0 then
    if not DelayOrEvent(Ticks div 2) then
    for i:=(bonus div 500-1) downto 0 do
    begin
      str(i*500,s);
      XText(TextWindowX+124,TextWindowY+90,false,VisiblePageOffs,s+'    ');
      AddtoScore(500);
      PlaySound(countdown);
      if DelayOrEvent(Ticks div 12) then
	if i<>0 then
	begin
	  AddtoScore((i-1)*500);
	  i:=1;
	end;
    end
    else
    begin
      XText(TextWindowX+124,TextWindowY+90,false,VisiblePageOffs,'0    ');
      AddtoScore(bonus);
      if bonus>0 then PlaySound(countdown);
    end;
    DelayOrEvent(longint(Ticks)*5 div 2);
  end;
end;

procedure ShowGameClockedMessage;
begin
  with GameInfo[Player] do
  begin
    XWindow(50+VisiblePageX,40+VisiblePageY,270+VisiblePageX,160+VisiblePageY,2,VisiblePageOffs);
    XTextCenter(160+VisiblePageX,TextWindowY+15,false,VisiblePageOffs,'CONGRATULATIONS');
    XTextCenter(160+VisiblePageX,TextWindowY+40,false,VisiblePageOffs,'YOU HAVE ATTAINED THE');
    XTextCenter(160+VisiblePageX,TextWindowY+65,false,VisiblePageOffs,'RANK OF');
    XTextCenter(160+VisiblePageX,TextWindowY+90,false,VisiblePageOffs,GameClockedNames[GameClocked]);
    WaitForEvent(NoStars);
  end;
end;

{$IFDEF BONUSLEVEL}
procedure GiveBonusLevelBonus;
var TimeTaken,Bonus,i:longint;
    s,s2:string[40];
begin
  with GameInfo[Player] do
  begin
    XPageFlip(VisiblePageX,VisiblePageY);
    with ship do XPutPBM(oldx,oldy,VisiblePageOffs,backgr^);
	      {erase ship}
    TimeTaken:=GameInfo[Player].TimeOnLevel div FrameRate;
    XWindow(55+VisiblePageX,45+VisiblePageY,250+VisiblePageX,155+VisiblePageY,2,VisiblePageOffs);
    XText(TextWindowX+50,TextWindowY+15,true,VisiblePageOffs,'BONUS LEVEL');
    str(levels[level].NumCryst:3,s);
    XText(TextWindowX+20,TextWindowY+40,true,VisiblePageOffs,'NO OF CRYSTALS: '+s);
    str(BonusLevelCount:3,s);
    XText(TextWindowX+20,TextWindowY+60,true,VisiblePageOffs,'NO COLLECTED:   '+s);
      if BonusLevelCount<(levels[level].NumCryst-2) then
				    begin
				      bonus:=BonusLevelCount*1000;
				      s:='BONUS:       ';
				    end;
      if BonusLevelCount=(levels[level].NumCryst-2) then
				    begin
				      bonus:=25000;
				      s:='JACKPOT:     ';
				    end;
      if BonusLevelCount=(levels[level].NumCryst-1) then
				    begin
				      bonus:=50000;
				      s:='JACKPOT:     ';
				    end;
      if BonusLevelCount=(levels[level].NumCryst) then
				    begin
				      bonus:=100000;
				      s:='SUPER JACKPOT:';
				    end;
    str(bonus:6,s2);
    Xtext(TextWindowX+20,TextWindowY+80,true,VisiblePageOffs,s+s2);
    AddToScore(bonus);
    DelayOrEvent(longint(Ticks)*3);
  end;
end;
{$ENDIF}


function ReadDefaults:boolean;
var i:integer;
    pl:PlayerType;
    k:KeysType;
begin
  if exist('xquest.cfg') then
  begin
{$I-}
    Assign(f,'xquest.cfg');
    reset(f);
    readln(f,SoundVolume);
    readln(f,i);
    if (i=2) then GameMode:=TwoPlayer else GameMode:=OnePlayer;
    for pl:=Player1 to Player2 do
    with PlayerInfo[pl] do
    begin
      readln(f);readln(f);
      readln(f,HInputSpeed);
      readln(f,VInputSpeed);
      readln(f,DiffLevel);
      readln(f,i);
      InputDevice:=InputDeviceType(lo(i));
      readln(f,MouseFireButton);
      readln(f,MouseSmartBombButton);
      readln(f,JoyFireButton);
      readln(f,JoySmartBombButton);
      for k:=UpKey to SmartBombKey do read(f,KeyArray[k]);
      readln(f);
    end;
    readln(f);
    with JoyCal[1] do
    readln(f,XMin,XCentreMin,XCentreMax,XMax,
	     YMin,YCentreMin,YCentreMax,YMax);
    readln(f,i);
    JoyStickACalibrated:=boolean(lo(i));
    readln(f,SoundCard);
    readln(f,SbAddr);
    readln(f,SbIrq);
    readln(f,SbDMA);
    readln(f,MaxSoundEffects);
    close(f);
    ReadDefaults:=(ioresult=0);
  end else ReadDefaults:=false;
end;

procedure WriteDefaults;
var pl:PlayerType;
    k:KeysType;
begin
  assign(f,'xquest.cfg');          {save defaults}
{$I-}
  rewrite(f);
  writeln(f,SoundVolume:4,' Sound Volume');
  if GameMode=OnePlayer then
    writeln(f,'   1 Number of Players')
  else
    writeln(f,'   2 Number of Players');
  for pl:=Player1 to Player2 do
  with PlayerInfo[pl] do
  begin
    writeln(f);
    if pl=Player1 then writeln(f,'Player One') else writeln(f,'Player Two');
    writeln(f,HInputSpeed:4,' Horizontal Input Sensitivity');
    writeln(f,VInputSpeed:4,' Vertical Input Sensitivity');
    writeln(f,DiffLevel:4,' Difficulty Level');
    writeln(f,ord(InputDevice):4,' InputDevice');
    writeln(f,MouseFireButton:4,' Mouse Fire Button');
    writeln(f,MouseSmartBombButton:4,' Mouse Smartbomb Button');
    writeln(f,JoyFireButton:4,' Joystick Fire Button');
    writeln(f,JoySmartBombButton:4,' Joystick Smartbomb Button');
    for k:=UpKey to SmartBombKey do
      write(f,KeyArray[k],' ');
    writeln(f,' Keys');
  end;
  writeln(f);
  with JoyCal[1] do
  writeln(f,XMin,' ',XCentreMin,' ',XCentreMax,' ',XMax,' ',
	    YMin,' ',YCentreMin,' ',YCentreMax,' ',YMax,' Joystick calibration values');
  writeln(f,ord(JoyStickACalibrated):4,' Joystick calibrated?');
  writeln(f,SoundCard:4,' Sound card');
  writeln(f,SbAddr:4,' Port');
  writeln(f,SbIrq:4,' IRQ');
  writeln(f,SBDMA:4,' DMA');
  writeln(f,MaxSoundEffects:4,' Maximum simultaneous sounds');
  close(f);
{$I+}
end;

procedure Initialise;

  procedure ParamHelp;
  begin
    writeln;
    writeln(' XQUEST [-nosound] [-slow]');
    writeln;
    writeln(' -nosound:         Turn off game sounds');
    writeln(' -slow:            Use this if you are using a slow computer and the');
    writeln('                   game starts dropping frames');
    writeln;
{$IFDEF GUS}
    writeln('Note that if you have a Gravis UltraSound card then the ULTRASND');
    writeln('  environment variable must be set properly.');
    writeln;
    writeln('See the manual for more details of setting up the sound correctly');
    writeln;
{$ENDIF}
    halt;
  end;

  procedure IncorrectParams;
  begin
    writeln;
    writeln('Unknown or incorrect parameters.');
    ParamHelp;
  end;

var i,ParamNum:integer;
    NoSound:boolean;

    dir:dirstr;nam:namestr;ext:extstr;s:string;
begin
{$IFDEF RECORDCHECK}
	  {debugging check on record sizes}
  writeln('enemytype: ',SizeOf(enemytype),'=',SizeOfEnemyType,' OK? missiletype: ',SizeOf(missiletype),'=30. OK?');
  writeln('emissiletype: ',Sizeof(emissiletype),'=',SizeOfEmissileType,' OK?');
  writeln('GameType: ',Sizeof(GameInfoType),'=',SizeOfGameInfoType,' OK?');
  ch:=readkey;if ch=#27 then halt;
{$ENDIF}

  new(demo);
  demofilename:='';
  recording:=false;
  demomode:=false;
  nosound:=false;
  MaxSoundEffects:=4;
  DemoFileName:='xquest.dmo';
{$IFDEF TESTINIT}
  if ReadDefaults then writeln('Defaults successfully read from XQUEST.CFG');
  readkey;
{$ELSE}
  if not ReadDefaults then
  begin
    if ParseBlaster(SBAddr,SBIrq,SBDMA) then SoundCard:=SoundBlaster;
  end;
{$ENDIF}
  for paramnum:=1 to paramcount do
  begin
    if paramstr(paramnum)='-nosound' then
    begin
      nosound:=true;
    end
    else
    if paramstr(paramnum)='-slow' then
    begin
      MaxSoundEffects:=2;
      CurrMaxMissiles:=20;
      CurrMaxEnemyMissiles:=45;    {limit max. no. of sprites for speed}
    end else

{$IFDEF GODMODE}
    if paramstr(paramnum)='-yakaboo' then
    begin
      StartLives:=99;
      StartBombs:=99;
    end else
{$ENDIF}

    if (paramstr(paramnum)='-?') or (paramstr(paramnum)='/?') or
       (paramstr(paramnum)='-h') or (paramstr(paramnum)='-help') or
       (paramstr(paramnum)='/help') or (paramstr(paramnum)='/h')  then
	 ParamHelp
    else
    IncorrectParams;
  end;

{$IFDEF GUS}
  if SoundCard=GUS then            {check for GUS card}
  if TestAndInitialiseGUS then
    GusPresent:=true
  else
    GUSPresent:=false;

  if not GusPresent then
{$ENDIF}

  if (not NoSound) and (SoundCard=SoundBlaster) then
  begin
    if SBInit(false)<>0 then
    begin
      writeln('Unable to reset Soundblaster DSP. Sound disabled.');
      repeat until keypressed;
      SoundCard:=NoSoundCard;
    end;
{$IFDEF TESTINIT}
    writeln('SoundBlaster initialised. Port:',SBAddr,' IRQ:',SBIrq,' DMA:',SBDMA);
    readkey;
{$ENDIF}
    SetSBVolume(SoundVolume);
{$IFDEF TESTINIT}
    writeln('SoundBlaster volume level set.');
    readkey;
{$ENDIF}
  end;
  SetExitProcedure;
{$IFDEF TESTINIT}
  writeln('Exit chain initialised.');
  readkey;
{$ENDIF}
  MouseSetup;
{$IFDEF TESTINIT}
  writeln('Mouse setup with defaults.');
  readkey;
{$ENDIF}

  if not MousePresent then
  begin
    PlayerInfo[Player1].InputDevice:=KeyboardInput;
    PlayerInfo[Player2].InputDevice:=KeyboardInput;
  end;
{$IFDEF GUS}    {debugging}
  writeln(SoundBlasterPresent,' ',GUSPresent,' ',SoundBlasterInitialised,' ',
	  SoundVolume,' ',MaxSoundEffects,' ',MaxSounds);
  readln;
{$ENDIF}

end;

procedure CheckMemory;
begin
{$IFDEF TESTMEM}
  writeln(memfile,'NEW GAME: Starting check memory: ',maxavail);
{$ENDIF}
  if memavail<MemoryRequired then
  begin
    writeln;
    writeln('XQuest cannot run in this amount of DOS memory. You need at least');
    writeln((MemoryRequired-memavail) div 1000 +1,' kbytes more low memory. Try removing some device drivers or TSRs,');
    writeln('running XQuest without sound (using the -nosound parameter) or if all');
    writeln('else fails run XQuest from a clean boot.');
    halt;
  end;
  if memavail<MemoryRecommended then
  begin
    writeln;
    writeln('Warning: XQuest can just about run with this amount of memory, but really');
    writeln('requires more. ',(MemoryRecommended-memavail) div 1000 +1,' kbytes more low memory are recommended.');
    writeln;
    writeln('XQuest may crash on the higher levels if you run it with this much');
    writeln('memory. Do you want to go ahead anyway (y/n)?');
    writeln;
    if yesno('Y')<>'Y' then halt;
  end;
end;


procedure InitialiseGame;
begin

  InitialiseEnemies;
{$IFDEF TESTINIT}
  writeln('Enemy constants loaded.');
  readkey;
{$ENDIF}
  InitialiseVariables;
{$IFDEF TESTINIT}
  writeln('Game variables initialised.');
  readkey;
{$ENDIF}
  if SoundBlasterInitialised then InitialiseSounds;
{$IFDEF TESTINIT}
  if SoundBlasterInitialised then
  begin
    writeln('Memory allocated for sounds and soundfile loaded.');
    readkey;
  end;
{$ENDIF}
  CheckMemory;
  SetXMode;             {needed for InitialiseGraphics, below}
  xputpalstruc(palette);
  TitlePage;            {large temporary memory use}
{$IFDEF TESTINIT}
  XTextMode;
  writeln('Title screens completed. About to initialise graphics and start game...');
  readkey;
  SetXModeNoSplitScreen;
  xputpalstruc(palette);
{$ENDIF}
  InitialiseGraphics;
end;   {InitialiseGame}

function SetDemoFile(demofilename:string):boolean;
begin
  {$I-}
  close(demofile);
  {$I+}
  if ioresult<>0 then begin end;  {clear ioresult}
  {$I-}
  assign(demofile,demofilename);
  reset(demofile,1);
  blockread(demofile,SavedRandSeed,4);
  SavedGameMode:=GameMode;
  blockread(demofile,GameMode,sizeof(GameModeType));
  blockread(demofile,DemoPlayerInfo,sizeof(PlayerInfoType));
  {$I+}
  DemoFileLoaded:=false;
  if ioresult=0 then
  begin
    demoparts:=(filesize(demofile)-filepos(demofile)) div (MaxDemoFrames*sizeof(demorectype));
    if demoparts>0 then blockread(demofile,demo^,MaxDemoFrames*sizeof(demoRecType))
		   else blockread(demofile,demo^,filesize(demofile)-filepos(demofile));
    SetDemofile:=true;
    DemoFileLoaded:=true;
  end else Setdemofile:=false;
end;    {SetDemoFile}

procedure MoveGate;
var i,move:integer;
begin
  move:=GateMoveCount div 64;
  GateMoveCount:=GateMoveCount mod 64;
    {carryover for next gate movement}
  with GameInfo[Player] do
    if random<levels[level].GateChangeDirProb then GateMovePos:=not GateMovePos;
  if move>0 then
  begin
    if GameInfo[Player].GateMovePos then
    begin
      inc(Gate[Left].X,move);inc(Gate[Right].X,move);
      if Gate[Right].X>MaxGateX then GameInfo[Player].GateMovePos:=false;
    end
    else
    begin
      dec(Gate[Left].X,move);dec(Gate[Right].X,move);
      if Gate[Left].X<MinGateX then GameInfo[Player].GateMovePos:=true;
    end;
(****
    XPutPBM(GateMinX -8,0,Page0Offs,Gate[Left]^);
    XPutPBM(GateMaxX-3,0,Page0Offs,Gate[Right]^);
    XPutPBM(GateMinX -8,0,Page1Offs,Gate[Left]^);
    XPutPBM(GateMaxX-3,0,Page1Offs,Gate[Right]^);
      {currently set up so that these do not need to be separately erased}
****)
    if GameInfo[Player].NumCrystals>0 then {barrier}
    begin
      XLine(Gate[Left].X+Gate[Left].Width,7,Gate[Right].X-1,7,Page0Offs,54);
      XLine(Gate[Left].X+Gate[Left].Width,7,Gate[Right].X-1,7,Page1Offs,54);
    end;
  end;
end;      {MoveGate}

procedure FixEMines(OX,OY,OXbr,OYbr:integer;Page:word);
var i:integer;
begin
  for i:=NumEnemyMines downto 1 do
  with emines.pos[i] do
    if (Xbr>=OX) and (Ybr>=OY) and
       (X<=OXbr) and (Y<=OYbr)
    then XPutCBitmap(X,Y,Page,emines.pic^);
end;    {FixEmines}

procedure AddEnemy(XC,YC:integer;i:integer);
var theta:real;
begin
  if NumEnemies>=MaxEnemies then exit;
  inc(NumEnemies);
  with enemy[NumEnemies] do
  begin
    typ:=enemykind[i];
    ntyp:=i;
    with typ do
    begin
      if (not maxspeed) then
      begin
	if (speed>0) then
	begin
	  delx:=(random(speed)-speed2)*gamespeed div 64;
	  dely:=(random(speed)-speed2)*gamespeed div 64;
	end else
	begin
	  delx:=0;
	  dely:=0;
	end;
	if i=1 then
	  supertime:=enemykind[1].numframes*(256 div enemykind[1].framespeed)
	  else supertime:=0;
	  {supertime measures correct no. of display frames for explosions}
      end
      else
      begin
	theta:=cos(random*2*pi-pi);
	delx:=round(speed*theta)*gamespeed div 64;
	dely:=round(speed*(1-theta*theta))*gamespeed div 64;
      end;

      if i=0 then supertime:=SuperTimeMin+random(SuperTimeRan);
	      {Set supercrystal timeout}
      curvesin:=-curve2+random(curve);
      curvecos:=round(sqrt(1-sqr(curvesin/32767))*32767);
      x:=XC;
      y:=YC;
      xbr:=x+width-1;
      ybr:=y+height-1;
      sx:=x shl 6;sy:=y shl 6;
      oldx:=x;oldy:=y;
      frame:=0;
      hit:=hits;
      getmem(backgr,bmwidth*4*height+4);
      getmem(oldbackgr,bmwidth*4*height+4);
{$IFDEF TESTMEM}
      writeln(memfile,'Enemy allocation: ',(bmwidth*4*height+4)*2,' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
      XGetPBM(x,y,bmwidth,height,HiddenPageOffs,backgr^);
      XGetPBM(x,y,bmwidth,height,HiddenPageOffs,oldbackgr^);
    end;
  end;
end;  {AddEnemy}


procedure AddEnemyMissile(en:integer);
var temp:integer;
begin
  if NumEnemyMissiles<CurrMaxEnemyMissiles then
  begin
    inc(NumEnemyMissiles);
    with emissiles[NumEnemyMissiles] do
    with enemy[en].typ do
    begin
      sx:=enemy[en].sx+(width shl 5);
      sy:=enemy[en].sy+(width shl 5);  {starting coordinates}
      x:=sx shr 6;y:=sy shr 6;
      oldx:=x;oldy:=y;
      mtyp:=emisskind[firetype];
      with mtyp do
      begin
	xbr:=x+mtyp.width-1;
	ybr:=y+mtyp.height-1;
	oldx:=x;oldy:=y;
	XGetMissPBM(x,y,{bmwidth,}height,HiddenPageOffs,backgr^);
	XGetMissPBM(x,y,{bmwidth,}height,HiddenPageOffs,oldbackgr^);
	PlaySound(soundnum);
	if FireDirect then
	begin
	  delx:=ship.x-x;
	  dely:=ship.y-y;
	  if (abs(delx)>abs(dely)) then
	    temp:=(abs(delx)+abs(dely) div 2)
	  else
	    temp:=(abs(dely)+abs(delx) div 2);
	  delx:=longint(delx)*mspeed div temp *gamespeed div 64;
	  dely:=longint(dely)*mspeed div temp *gamespeed div 64;
	    {mspeed, in direction of ship}
	end
	else
	begin   {not truly random fire, but close enough}
	  delx:=(random(mspeed)-(mspeed shr 1))*gamespeed div 64;
	  dely:=(random(mspeed)-(mspeed shr 1))*gamespeed div 64;
	end;
      end;
    end;
  end;
end;            {AddEnemyMissile}

procedure Explode(en:integer);
var i,temp:integer;
begin
  temp:=CurrMaxEnemyMissiles-NumEnemyMissiles;
  if temp>15 then temp:=15;
  for i:=1 to temp do
  begin
    inc(NumEnemyMissiles);
    with emissiles[NumEnemyMissiles] do
    with enemy[en].typ do
    begin
      sx:=enemy[en].sx+(longint(enemy[en].typ.width) shl 5);
      sy:=enemy[en].sy+(longint(enemy[en].typ.width) shl 5);  {starting coordinates}
      x:=sx shr 6;
      y:=sy shr 6;
      oldx:=x;oldy:=y;
      mtyp:=emisskind[firetype];
      with mtyp do
      begin
	xbr:=x+mtyp.width-1;
	ybr:=y+mtyp.height-1;
	XGetMissPBM(x,y,{bmwidth,}height,HiddenPageOffs,backgr^);
	XGetMissPBM(x,y,{bmwidth,}height,HiddenPageOffs,oldbackgr^);
	PlaySound(soundnum);
	delx:=(longint(cost[i])*mspeed) div 32768 *gamespeed div 64;
	dely:=(longint(sint[i])*mspeed) div 32768 *gamespeed div 64;
      end;
    end;
  end;
end;            {Explode}

procedure DeleteEnemy(i:integer);
var temp:enemytype;
begin
  with enemy[i] do
  with typ do
  begin
    XPutPBM(oldx,oldy,VisiblePageOffs,oldbackgr^);
    freemem(backgr,bmwidth*4*height+4);
    freemem(oldbackgr,bmwidth*4*height+4);
{$IFDEF TESTMEM}
      writeln(memfile,'Enemy deallocation: ',(bmwidth*4*height+4)*2,' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
  end;
  temp:=enemy[NumEnemies];
  enemy[NumEnemies]:=enemy[i];
  enemy[i]:=temp;
  dec(NumEnemies);
end;

procedure EnemyDestroyed(i,dx,dy:integer;explosion:boolean);
{i is the number of the destroyed enemy
 dx,dy is the velocity of the missile (if any) that destroyed that enemy
 explosion determines whether an  explosion is shown or not
}
var j,k:integer;

begin
  if (PowerUp[HeavyFire].value>0) then
  begin
    enemy[i].hit:=0;            {heavyfire kills all enemies}
  end else dec(enemy[i].hit);
  if (enemy[i].hit>0) then
  begin
    PlaySound(doh);
  end
  else
  begin
    with enemy[i] do
    with typ do
    begin
      if shootback then AddEnemyMissile(i);
      if rebounds and (powerup[HeavyFire].value<=0) then
      begin
	playsound(ow);
	explosion:=false;
      end;
      if explodes then
      begin
	Explode(i);
      end;
      AddToScore(score);
    end;
    if explosion then
    begin
      if enemy[i].typ.DeathSound=explosn then
	PlaySound(enemy[i].typ.DeathSound+random(3))
	      {random of 3 explosion sounds}
      else
	PlaySound(enemy[i].typ.DeathSound);

      if enemy[i].typ.tribbles then  {if tribbles then add tribbles}
      with enemy[i] do
      begin
	AddEnemy(x,y,ntyp+1);
	AddEnemy(x,y,ntyp+1);
	AddEnemy(x,y,ntyp+1);
	AddEnemy(x,y,ntyp+1);
	AddEnemy(x,y,ntyp+1);
      end;
      j:=enemy[i].x;
      k:=enemy[i].y;
      DeleteEnemy(i);
      AddEnemy(j,k,1);
    end
    else
    if enemy[i].typ.rebounds and (powerup[HeavyFire].value<=0) then
    begin
      enemy[i].delx:=3*enemy[i].delx div 4 + dx div 4;
      enemy[i].dely:=3*enemy[i].dely div 4 + dy div 4;
    end
    else DeleteEnemy(i);
  end;
end;       {EnemyDestroyed}


procedure FireSmartBomb;
var i:integer;
begin
  if GameInfo[player].NumSmartBombs>0 then
  begin
    for i:=NumEnemies downto 1 do
    begin
      enemy[i].hit:=1;          {kill all enemies}
      if enemy[i].ntyp>1 then
	EnemyDestroyed(i,0,0,true)
      else
	EnemyDestroyed(i,0,0,false);
    end;
    for i:=NumEnemyMissiles downto 1 do
    with emissiles[i] do
    with mtyp do
      XPutMissPBM(oldx,oldy,VisiblePageOffs,oldbackgr^);
    NumEnemyMissiles:=0;          {erase missiles}
    dec(GameInfo[Player].NumSmartBombs);
    with SmartBombPal[10] do XSetRGB(0,R,G,B);    {flash screen}
    SmartBombed:=11;
    PlaySound(explosn);
  end;
  ShowSmartBombs;                               {display new #smarts}
end;  {FireSmartBomb}

function CollideBitmaps(mask1:maskptr;height1,x1,y1:integer;
			 mask2:maskptr;height2,x2,y2:integer):boolean;
  {checks bitmap collisions. mask1 is the left hand mask and the
    bounding boxes must have collided
     ie x1<=x2 and 0<=x2-x1<=31}
  const bits:array[0..31] of longint=
    ($8000,$4000,$2000,$1000,$800,$400,$200,$100,$80,$40,$20,$10,$8,$4,$2,$1,
     $80000000,$40000000,$20000000,$10000000,$8000000,$4000000,$2000000,
     $1000000,$800000,$400000,$200000,$100000,$80000,$40000,$20000,$10000);

  var i,j:integer;
{$IFDEF TESTCOLLIDE}
      origmask1,origmask2:masktype;
{$ENDIF}

begin
{$IFDEF TESTCOLLIDE}
    origmask1:=mask1^;
    origmask2:=mask2^;
{$ENDIF}
    asm
	push    ds
	mov     cx,x2
	sub     cx,x1         {0<=cx<=31, difference in x-coords}

	mov     ax,word ptr [height1]   {loop counter}

	lds     si,[mask1]
	les     di,[mask2]    {ds:si and es:di point to the two masks}
	mov     ax,[y2]
	sub     ax,[y1]       {ax is set to the difference in the y coords}
	jl      @Mask2Upper   {which sprite has a lower y coord?}

{Mask 1 uppermost:}
	mov     dx,[height1]
	sub     dx,ax         {# of dwords of overlap of first mask with second}
			      {  ie number of rows to be compared}
	cmp     dx,[height2]  {will this run below the end of the second sprite?}
	jle     @HeightOK1
	mov     dx,[height2]  {Yes, set number of rows to be compared to the height of sprite 2}
@HeightOK1:
	shl     ax,2          {# of dwords to skip in first mask}
	add     si,ax         {si now points to top of overlap of 1st mask
			       with second}
	cmp     cx,16
	jge     @OLoop16

@OverlayLoop:  {assumes cl < 16}
	mov     bx,word ptr ds:[si+2]
	mov     ax,16
	sub     ax,cx
	xchg    cx,ax
	shr     bx,cl
	xchg    cx,ax
	mov     ax,word ptr ds:[si]
	shl     ax,cl
	or      ax,bx
	and     ax,word ptr es:[di]
	jnz     @collision

	mov     ax,word ptr ds:[si+2]
	shl     ax,cl
	and     ax,word ptr es:[di+2]
	jnz     @collision

	add     si,4
	add     di,4          {otherwise, check the next mask entries}
	dec     dx            {Any more rows to be checked?}
	jnz     @overlayloop
	jmp     @nocollision  {No, report no collision}

@OLoop16:  {cx > 16}
	sub     cx,16
@OLoop:
	mov     ax,word ptr ds:[si+2]
	shl     ax,cl
	and     ax,word ptr es:[di]
	jnz     @collision

	add     si,4
	add     di,4          {otherwise, check the next mask entries}
	dec     dx            {Any more rows to be checked?}
	jnz     @OLoop
	jmp     @nocollision  {No, report no collision}

@Mask2Upper:                  {Mask 2 is uppermost}
	neg     ax
	mov     dx,[height2]
	sub     dx,ax         {# of dwords of overlap of second mask with first}
			      {  ie number of rows to be compared}
	cmp     dx,[height1]  {will this run below the end of the first sprite?}
	jle     @HeightOK2
	mov     dx,[height1]  {Yes, set number of rows to be compared to the height of sprite 1}
@HeightOK2:
	shl     ax,2          {# of dwords to skip in second mask}
	add     di,ax         {di now points to top of overlap of 2nd mask
			       with first}
	jmp     @OverlayLoop

@Collision:
{$IFDEF TESTCOLLIDE}
	mov  [i],1
{$ENDIF}
	mov    @Result,1
	jmp    @finished

@NoCollision:
{$IFDEF TESTCOLLIDE}
	mov [i],0
{$ENDIF}
	mov    @Result,0
@finished:
	pop    ds
  end;

{$IFDEF TESTCOLLIDE}
  if i=1 then
  begin
      writeln(debugfile,x1,' ',y1,' ',x2,' ',y2);
      for i:=0 to (height1-1) do
      begin
	for j:=0 to 31 do if (mask1^[i] and (bits[j]))=0 then
	  write(debugfile,'0') else write(debugfile,'1');
	writeln(debugfile);
      end;
      writeln(debugfile);
      for i:=0 to (height2-1) do
      begin
	for j:=0 to 31 do if (mask2^[i] and (bits[j]))=0 then
	  write(debugfile,'0') else write(debugfile,'1');
	writeln(debugfile);
      end;
      writeln(debugfile);
      for i:=0 to (height1-1) do
      begin
	for j:=0 to 31 do if (origmask1[i] and (bits[j]))=0 then
	  write(debugfile,'0') else write(debugfile,'1');
	writeln(debugfile);
      end;
      writeln(debugfile);
      for i:=0 to (height2-1) do
      begin
	for j:=0 to 31 do if (origmask2[i] and (bits[j]))=0 then
	  write(debugfile,'0') else write(debugfile,'1');
	writeln(debugfile);
      end;
      writeln(debugfile);
      writeln(debugfile,height1);
      for i:=0 to (height1-1) do writeln(debugfile,origmask1[i]);
      writeln(debugfile);
      writeln(debugfile,height2);
      for i:=0 to (height2-1) do writeln(debugfile,origmask2[i]);
    end;
{$ENDIF}

end;


(*  Alternative faster 386 code
begin
    asm
	mov     cx,x2
	sub     cx,x1         {0<=cx<=31, difference in x-coords}

	mov     ax,word ptr [height1]   {loop counter}

	lea     si,mask1
	lea     di,mask2
	mov     ax,[y2]
	sub     ax,[y1]
	jl      @Mask2Upper

	mov     dx,[height1]
	sub     dx,ax   {# of dwords of overlap of first mask}
	shl     ax,2    {# of dwords to skip in first mask}
	add     si,ax   {si now points to top of overlap of 1st mask
			 with second}

@OverlayLoop:
	db      $66
	mov     ax,word ptr ss:[si]   {mov eax, dword ptr ss:[si]}
	db      $66
	shl     ax,cl                 {shl eax, cl}
	db      66h
	and     ax,word ptr ss:[di]    {and eax, dword ptr ss:[si]}
	jnz     @collision


	add     si,4
	add     di,4   {next mask entries}
	dec     dx
	jnz     @overlayloop
	jmp     @nocollision

@Mask2Upper:
	neg     ax
	mov     dx,[height2]
	sub     dx,ax  {# of dwords of overlap of second mask}
	shl     ax,2   {# of dwords to skip in second mask}
	add     di,ax  {di now points to top of overlap of 2nd mask
			with first}
	jmp     @OverlayLoop

@Collision:
	mov    @Result,1
	jmp    @finished

@NoCollision:
	mov    @Result,0
@finished:
  end;
end;
*)

{$IFDEF TESTCOLLIDE}
procedure TestCollideBitmaps;  {ship vs. meeby}
const Mask1:MaskType=(16382,16383,8191,8191,4095,8191,8190,8190,4094,
		      8188,16376,32764,32766,32766,32766,16380,4088,
		      0,0,0,0,0,0,0);
      Mask2:MaskType=(3968,16352,16352,32752,32752,32752,32752,32752,
		      16352,16352,3968,0,0,0,0,0,0,0,0,0,0,0,0,0);
var res:boolean;

begin
  res:=CollideBitmaps(@Mask1,20,100,100,@Mask2,11,110,110);
end;
{$ENDIF}


procedure MoveShip;
var but,missbut,smartbut,mx,my,temp:integer;
    theta:real; {For ShipDir}
    deltax,deltay,modulus:longint; {for Gravity/repulsor effects}
    i:longint;
{    r,theta:real;}
{    s:string[20]; {DEBUG}

  procedure FireMissile(deltax,deltay:integer);
  begin
    if ship.NumMissiles<CurrMaxMissiles then
    begin
      inc(ship.NumMissiles);
      with missiles[ship.NumMissiles] do
      begin
	sx:=ship.sx+ship.width shl 5 - ship.misswidth shl 4;
	sy:=ship.sy+ship.height shl 5 - ship.missheight shl 4;
	   {missile centre on ship centre}
	x:=sx shr 6;
	y:=sy shr 6;
	xbr:=x+ship.misswidth-1;
	ybr:=y+ship.missheight-1;
	delx:=deltax;
	dely:=deltay;    {double ship's speed}
	oldx:=x;oldy:=y;
	time:=0;
	XGetMissPBM(x,y,ship.missheight,HiddenPageOffs,backgr^);
	XGetMissPBM(x,y,ship.missheight,HiddenPageOffs,oldbackgr^);
      end;
    end;
  end;   {FireMissile}

  procedure Shoot(deltax,deltay:integer);
  const shotdelta=pi/180*10; {10 degrees}
	AimedFireSpeed=256;
  var r:real;
      i,j,exp,eyp,framedelta:integer;
      dist,mindist:longint;
  begin
    if ship.NumMissiles<CurrMaxMissiles then PlaySound(fire);
    if (PowerUp[AimedFire].value>0) and (NumEnemies>0) then
    begin
      i:=1;j:=1;mindist:=maxlongint;
      while i<=NumEnemies do
      begin
	dist:=(sqr(longint(ship.x-enemy[i].x))+sqr(longint(ship.y-enemy[i].y)));
	if dist<mindist then
	begin
	  mindist:=dist;
	  j:=i;
	end;
	inc(i);
      end;
      mindist:=round(sqrt(mindist));
      framedelta:=mindist div (AimedFireSpeed div 64);
	 {est. no. of frames for missile to reach enemy}
      exp:=enemy[j].x+(framedelta*enemy[j].delx) div 64;
      eyp:=enemy[j].y+(framedelta*enemy[j].dely) div 64;
	 {expected enemy x and y positions after this no. of frames}
      deltax:=(AimedFireSpeed*longint(exp-ship.x)) div mindist;
      deltay:=(AimedFireSpeed*longint(eyp-ship.y)) div mindist;
    end;
    FireMissile(deltax,deltay);
    if powerup[AssFire].value>0 then FireMissile(-deltax,-deltay);
    if powerup[MultiFire].value>0 then
    begin
      r:=sqrt(sqr(longint(deltax))+sqr(longint(deltay)));
      if deltax<>0 then theta:=arctan(deltay/deltax)
	else if deltay>0 then theta:=pi/2 else theta:=-pi/2;
      if deltax<0 then theta:=theta+3.14159;
      FireMissile(round(r*cos(theta+shotdelta)), round(r*sin(theta+shotdelta)));
      FireMissile(round(r*cos(theta-shotdelta)), round(r*sin(theta-shotdelta)));
      if PowerUp[AssFire].value>0 then
      begin
	FireMissile(-round(r*cos(theta+shotdelta)), -round(r*sin(theta+shotdelta)));
	FireMissile(-round(r*cos(theta-shotdelta)), -round(r*sin(theta-shotdelta)));
      end;
    end;
  end;


  procedure DecrementPowerUps;
  var PU:PowerUpType;
  begin
    for PU:=Shield to Bounce do
    with PowerUp[PU] do
    begin
      if value>0 then
      begin
	dec(value);
	if value<198 then
	if (value mod 22=0) and (position<>0) then
	  XRectFill(Position,5,Position+17,20,SplitScrnOffs,14)
	else if (value mod 22=11) and (position<>0) then
	  XPutPBM(Position,5,SplitScrnOffs,Pic^);
	if Value=0 then ShowPowerUps;
      end;
    end;
  end;

  function WallCollideHor(mask:masktype;height:integer;wallmask:longint):boolean;
  type longtoword=record
		    high,low:word;
		  end;
  var i:word;
      collided:boolean;
  const bits:array[0..31] of longint=
    ($8000,$4000,$2000,$1000,$800,$400,$200,$100,$80,$40,$20,$10,$8,$4,$2,$1,
     $80000000,$40000000,$20000000,$10000000,$8000000,$4000000,$2000000,
     $1000000,$800000,$400000,$200000,$100000,$80000,$40000,$20000,$10000);
  begin
    i:=longtoword(wallmask).high;
    longtoword(wallmask).high:=longtoword(wallmask).low;
    longtoword(wallmask).low:=i;
    collided:=false;
    for i:=0 to (height-1) do
      if (mask[i] and wallmask)<>0 then collided:=true;
    WallCollideHor:=collided;
(*
    readln;
      XTextMode;
      mask[height-1]:=wallmask;
      for i:=0 to (height-1) do
      begin
	for j:=0 to 31 do if (mask[i] and (bits[j]))=0 then
	  write('0') else write('1');
	writeln('  ',mask[i]);
      end;
      readln;
*)
  end;

  function WallCollideVer(mask:masktype;minheight,maxheight:integer):boolean;
  var i:integer;
      collided:boolean;
  begin
    collided:=false;
    for i:=(minheight) to (maxheight) do
      if (mask[i]<>0) then collided:=true;
    WallCollideVer:=collided;
  end;

  procedure DemoFrameInitialise;
  begin
    inc(demoptr);
{$IFDEF DEBUGDEMO}
    writeln(debugfile,'demoptr= ',demoptr);
{$ENDIF}
    if demoptr>MaxDemoFrames then
    begin
      i:=filesize(demofile)-filepos(demofile);
      if i>MaxDemoFrames*sizeof(demorectype)
	then i:=MaxDemoFrames*sizeof(demorectype);
{$IFDEF DEBUGDEMO}
      writeln(debugfile,'DemoFrameInitialise: i= ',i);
{$ENDIF}
      if i<=0 then
      begin
	XTextMode;
	writeln('Demo file error. Halting. Sorry!');
        WaitForEvent(NoStars);
	halt(255);
      end;
{$IFDEF DEBUGDEMO}
      writeln(debugfile,'DemoFrameInitialise: reading ',i,' records');
{$ENDIF}
      blockread(demofile,demo^,i);
      demoptr:=1;
    end;
  end;

  procedure EnemyRepel(i:integer);
  begin {*** repulsion effect}
    deltax:=ship.x-enemy[i].x;
    deltay:=ship.y-enemy[i].y;
    if (deltax<>0) or (deltay<>0) then {avoid divide by zero errors}
    begin
      if (abs(deltax)>abs(deltay)) then
	modulus:=( (sqr(deltax)+sqr(deltay))*(abs(deltax)+abs(deltay) div 2))
      else
	modulus:=( (sqr(deltax)+sqr(deltay))*(abs(deltay)+abs(deltax) div 2));
      if modulus>0 then
      begin
	if modulus<500000 then if (framecount and 31)=0 then PlaySound(repulse);
	ship.delx:=ship.delx+ (8192*deltax) div modulus;
	ship.dely:=ship.dely+ (8192*deltay) div modulus;
      end;
    end;
  end; {repulsion effect}

  procedure DoAttractor;
  begin {*** attraction effect after game clocked}
    with GameInfo[Player] do
    begin
      deltax:=ship.x-AttractorX;
      deltay:=ship.y-AttractorY;
      if (deltax<>0) or (deltay<>0) then {avoid divide by zero errors}
      begin
	if (abs(deltax)>abs(deltay)) then
	  modulus:=( (sqr(deltax)+sqr(deltay))*(abs(deltax)+abs(deltay) div 2))
	else
	  modulus:=( (sqr(deltax)+sqr(deltay))*(abs(deltay)+abs(deltax) div 2));
	if modulus>0 then
	begin
	  ship.delx:=ship.delx- (8192*deltax) div modulus;
	  ship.dely:=ship.dely- (8192*deltay) div modulus;
	end;
      end;
    end; {attraction effect after game clocked}
  end;

  procedure CheckWallCollisions;
  var temp:longint;
      GT:GateType;
  begin
    with Ship do
    begin
      if (xbr>=ShipMaxX-10) then
	if WallCollideHor(mask[ShipDir]^,height,longint(1) shl (31-(ShipMaxX-x))-1) then
	begin
	  x:=ShipMaxX-ship.width;
	  if (Powerup[Shield].value<=0) and (PlayerInfo[Player].DiffLevel>1)
	   then ShipDestroyed:=true
	    else delx:=-abs(delx);
	end;
      if (x<ShipMinX) then
	if WallCollideHor(mask[ShipDir]^,height,not(longint(1) shl (31-(ShipMinX-x-1))-1)) then
	begin
	  x:=ShipMinX;
	  if (Powerup[Shield].value<=0) and (PlayerInfo[Player].DiffLevel>1)
	   then ShipDestroyed:=true
	    else delx:=abs(delx);
	end;
{***}
      if (y<(PageHeight div 2 + 7)) and (ybr>(PageHeight div 2 -7)) then
	  begin
	    if x<ShipMinX+6 then
	    begin
	      x:=ShipMinX+7;
	      if (Powerup[Shield].value<=0) and (PlayerInfo[Player].DiffLevel>1)
	       then ShipDestroyed:=true
		else delx:=abs(delx);
	    end;
	    if xbr>ShipMaxX-6 then
	    begin
	      x:=ShipMaxX-7-ship.width;
	      if (Powerup[Shield].value<=0) and (PlayerInfo[Player].DiffLevel>1)
		then ShipDestroyed:=true
		else delx:=-abs(delx);
	    end;
	  end;  {checks for collision with enemy inlets}

      temp:=x-(VisiblePageX+ScrnPhysicalPixelWidth-ScreenHBorder);
		   {screen scrolling: horizontal}
      if (temp>0) and (VisiblePageX<MaxVisiblePageX) then
      begin
	inc(VisiblePageX,temp div 20 +1);
	if VisiblePageX>MaxVisiblePageX then VisiblePageX:=MaxVisiblePageX;
      end;
      temp:=(VisiblePageX+ScreenHBorder)-x;
      if (temp>0) and (VisiblePageX>0) then
      begin
	dec(VisiblePageX,temp div 20 +1);
	if VisiblePageX<0 then VisiblePageX:=0;
      end;

      if ybr>ShipMaxY then        {check Y coord}
	if WallCollideVer(mask[ShipDir]^,ShipMaxY-y+1,ship.height-1) then
	begin
	  y:=ShipMaxY-ship.height;
	  if (Powerup[Shield].value<=0) and (PlayerInfo[Player].DiffLevel>1)
	   then ShipDestroyed:=true
	    else dely:=-abs(dely);
	end;

	for GT:=Left to Right do
	with Gate[GT] do
	begin
	  if ((x+width-1)>=ship.x) and (x<=ship.xbr) and
	     ((height-1)>=ship.y)
	  then
	    if ((ship.x<=x) and
	      CollideBitmaps(ship.mask[Ship.ShipDir],ship.height,ship.x,ship.y,mask,height,x,0))
	    or ((x<=ship.x) and
	      CollideBitmaps(mask,height,x,0,ship.mask[ship.ShipDir],ship.height,ship.x,ship.y))
	  then
	  begin
	    y:=height;
	    if Powerup[Shield].value<=0 then ShipDestroyed:=true
	      else dely:=abs(dely);
	  end
	end;
      if y<ShipMinY then
      begin
	if (x<=Gate[Left].X) or (xbr>=Gate[Right].X+Gate[Right].Width) then
	  begin
	    if WallCollideVer(mask[ShipDir]^,0,ShipMinY-y-1) then
	    begin
	      y:=ShipMinY;
	      if (Powerup[Shield].value<=0) and (PlayerInfo[Player].DiffLevel>1)
		then ShipDestroyed:=true
		else dely:=abs(dely);
	    end
	  end
	  else
	  begin
	    if GameInfo[Player].NumCrystals>0 then
	    if WallCollideVer(mask[ShipDir]^,0,8-y-1) then
	    begin
	      y:=ShipMinY;
	      if (Powerup[Shield].value<=0) then ShipDestroyed:=true
		else dely:=abs(dely);
	    end;
	    if y<5 then if not ShipDestroyed then LevelFinished:=true;
	  end;
      end;
      temp:=y-(VisiblePageY+SplitScrnScanLine-ScreenVBorder);
			{vertical scrolling}
      if (temp>0) and (VisiblePageY<MaxVisiblePageY) then
      begin
	inc(VisiblePageY,temp div 20 +1);
	if VisiblePageY>MaxVisiblePageY then VisiblePageY:=MaxVisiblePageY;
      end;
      temp:=(VisiblePageY+ScreenVBorder)-y;
      if (temp>0) and (VisiblePageY>0) then
      begin
	dec(VisiblePageY,temp div 20 +1);
	if VisiblePageY<0 then VisiblePageY:=0;
      end;
    end;
  end;


begin       {MoveShip}
  if demomode then DemoFrameInitialise
  else
  if recording then
  begin
    inc(demoptr);
{$IFDEF DEBUGDEMO}
    writeln(debugfile,'demoptr= ',demoptr);
{$ENDIF}
    if demoptr>MaxDemoFrames then
    begin
      demoptr:=1;
{$IFDEF DEBUGDEMO}
    writeln(debugfile,'Update: demo block written.',MaxDemoFrames*sizeof(demorectype),' records');
{$ENDIF}
      blockwrite(demofile,demo^,MaxDemoFrames*sizeof(demorectype));
    end;
  end;
{$IFDEF DEBUG}
  if demoptr mod 100=1 then writeln(debugfile,'DemoPtr: ',demoptr,' Randseed: ',randseed);
{$ENDIF}
  with ship do InputMovement(delx,dely,true,false,false,true);
  if recording then
  begin
    demo^[demoptr].delx:=ship.delx;
    demo^[demoptr].dely:=ship.dely;
    demo^[demoptr].but:=0;
  end
  else
  if demomode then
  begin
    ship.delx:=demo^[demoptr].delx;
    ship.dely:=demo^[demoptr].dely;
  end;
  with ship do
  begin
    temp:=abs(delx)+abs(dely);
    if temp>MaxShipSpeed then     {limit speed}
    begin
      delx:=round(delx/temp*MaxShipSpeed);
      dely:=round(dely/temp*MaxShipSpeed);
    end;

    for i:=1 to NumEnemies do
    if enemy[i].typ.repulses then EnemyRepel(i);

    with GameInfo[Player] do
    if GameClocked>0 then DoAttractor;

{*** Update main ship info}
    sx:=sx+delx;sy:=sy+dely;
    oldx:=x;oldy:=y;
    x:=sx div 64;y:=sy div 64;
    xbr:=x+width-1;
    ybr:=y+height-1;

    if dely<>0 then theta:=arctan(delx/dely)
      else if delx>0 then theta:=pi/2 else theta:=-pi/2;
    if dely<0 then theta:=theta+3.14159;
    if theta<0 then theta:=theta+2*pi;

    ShipDir:=round(theta/(2*pi)*MaxShipPics);
    if ShipDir>=MaxShipPics then ShipDir:=ShipDir-MaxShipPics;

    CheckWallCollisions; {checks collisions and scrolls screen}

  end;

  DecrementPowerUps;

  but:=ButtonPressed;                             {check input}
  with PlayerInfo[Player] do
  begin
    smartbut:=(but and (MouseSmartBombButton or JoySmartBombButton));
    missbut:=(but and (MouseFireButton or JoyFireButton));
  end;
  but:=ButtonDown;

  if demomode then             {abort on button click}
  begin
    if (smartbut or missbut)<>0 then
    begin
      GameOver:=true;
      smartbut:=0;missbut:=0;
    end;
  end
  else
  if numkeypresses(PlayerInfo[Player].KeyArray[SmartBombKey])>0 then SmartBut:=1;
  if numkeypresses(PlayerInfo[Player].KeyArray[FireKey])>0 then MissBut:=1;

  if (missbut>0) or (demomode and ((demo^[demoptr].but and 1)=1)) then
  begin
    if recording then demo^[demoptr].but:=demo^[demoptr].but or 1;
    Shoot(ship.delx shl 1, ship.dely shl 1);
  end
  else
  if (powerup[RapidFire].value>0) then
    if (FrameCount and 3=0) then
      with PlayerInfo[Player] do
      if (but and (MouseFireButton or JoyFireButton)<>0) or
	 (demomode and ((demo^[demoptr].but and 4)<>0)) then
  begin
    if recording then demo^[demoptr].but:=demo^[demoptr].but or 4;
    Shoot(ship.delx shl 1, ship.dely shl 1);
  end;
  if (smartbut>0) or (demomode and ((demo^[demoptr].but and 2)=2)) then
  begin
    if recording then demo^[demoptr].but:=demo^[demoptr].but or 2;
    FireSmartBomb;
  end;
{$IFDEF DEBUGDEMO}
  if demomode or recording then
    with demo^[demoptr] do
    writeln(debugfile,'X: ',delx,' Y: ',dely,' But: ',but,' Randseed: ',randseed);
{$ENDIF}

end;     {MoveShip}

{***
Note: For the recording, the but number is interpreted as follows:
bit 0:  Missile button pressed
bit 1:  SmartBomb button pressed
bit 2:  Missile button held down
***}





procedure MoveEnemies;
var i,temp:integer;
    ltemp:longint;
    theta:real;

  procedure AddEnemyMine(en:integer);
  begin
    if (not Emines.adding) and (NumEnemyMines<MaxEnemyMines)  then
    begin
      inc(NumEnemyMines);
      with emines.pos[NumEnemyMines] do
      with enemy[en].typ do
      begin
	x:=enemy[en].x+(width shr 1);
	y:=enemy[en].y+(width shr 1);
	xbr:=x+emines.width-1;
	ybr:=y+emines.height-1;
	XPutCBitmap(x,y,HiddenPageOffs,emines.pic^);
	emines.adding:=true;
      end;
      PlaySound(squelch);
    end;
  end;      {AddEnemyMine}



begin     {MoveEnemies}
  for i:=0 to maxenemykinds do
    with GameInfo[Player] do
    with PlayerInfo[Player] do
    if (NumEnemies<levels[level].MaxEnemies) and
    (random<levels[level].erelease*DiffInfo[DiffLevel].enemyfrequency) and
    (random(100)<probs[level,i]) then
    begin
      if (random(2)=1) and (EnemyEnteringLeft<=0) then
      begin
	EnemyEnteringLeft:=80;
	EnemyLeftType:=i;
	XPutPBM(0,PageHeight div 2-10,Page0Offs,LEnemyGate[1]^);
	XPutPBM(0,PageHeight div 2-10,Page1Offs,LEnemyGate[1]^);
      end
      else if (EnemyEnteringRight<=0) then
      begin
	EnemyEnteringRight:=80;
	EnemyRightType:=i;
	XPutPBM(PageWidth-20,PageHeight div 2-10,Page0Offs,REnemyGate[1]^);
	XPutPBM(PageWidth-20,PageHeight div 2-10,Page1Offs,REnemyGate[1]^);
      end
    end;

  if EnemyEnteringLeft>0 then
  begin
    dec(EnemyEnteringLeft);
    if EnemyEnteringLeft and 7=0 then
    begin
      XPutPBM(0,PageHeight div 2-10,Page0Offs,LEnemyGate[(EnemyEnteringLeft shr 3) mod 5+1]^);
      XPutPBM(0,PageHeight div 2-10,Page1Offs,LEnemyGate[(EnemyEnteringLeft shr 3) mod 5+1]^);
    end;
    if EnemyEnteringLeft=0 then
    begin
      AddEnemy(15,EnemyStartY,EnemyLeftType);
      XPutPBM(0,PageHeight div 2-10,Page0Offs,LEnemyGate[0]^);
      XPutPBM(0,PageHeight div 2-10,Page1Offs,LEnemyGate[0]^);
      PlaySound(enemyent);
    end;
  end;

  if EnemyEnteringRight>0 then
  begin
    dec(EnemyEnteringRight);
    if EnemyEnteringRight and 7=0 then
    begin
      XPutPBM(PageWidth-20,PageHeight div 2-10,Page0Offs,REnemyGate[(EnemyEnteringRight shr 3) mod 5+1]^);
      XPutPBM(PageWidth-20,PageHeight div 2-10,Page1Offs,REnemyGate[(EnemyEnteringRight shr 3) mod 5+1]^);
    end;
    if EnemyEnteringRight=0 then
    begin
      AddEnemy(PageWidth-enemykind[EnemyRightType].width-16,EnemyStartY,EnemyRightType);
      XPutPBM(PageWidth-20,PageHeight div 2-10,Page0Offs,REnemyGate[0]^);
      XPutPBM(PageWidth-20,PageHeight div 2-10,Page1Offs,REnemyGate[0]^);
      PlaySound(enemyent);
    end;
  end;

  for i:=1 to NumEnemies do
  begin
    with enemy[i].typ do
    begin
      if fires and (random<fireprob) then AddEnemyMissile(i);
      if laysmines and (random<fireprob) then AddEnemyMine(i);
      with enemy[i] do
      begin
	inc(frame,framespeed);
	if frame>=((numframes+1) shl 8) then frame:=0;
		 { plot frames 0.. numframes }
	if (follows and (random<follow)) and (not ShipDestroyed) then
	begin
	  delx:=ship.x-x;
	  dely:=ship.y-y;
	  temp:=abs(delx)+abs(dely);
	  delx:=longint(delx)*speed div temp *gamespeed div 64;
	  dely:=longint(dely)*speed div temp *gamespeed div 64;
	    {dx + dy =speed, in direction of ship}
	end
	else
	if random<changedir then
	begin
	  if (zoom and ((abs(ship.delx)+abs(ship.dely))<60)) then
	  begin
	    PlaySound(bark);
	    delx:=ship.x-x;
	    dely:=ship.y-y;
	    temp:=abs(delx)+abs(dely);
	    delx:=delx*speed div temp *gamespeed div 48;
	    dely:=dely*speed div temp *gamespeed div 48;

	    {dx + dy =speed, in direction of ship}
	    {N.B. zoom is at 1.5 x normal speed}
	  end
	  else if not maxspeed then
	  begin
	    delx:=(random(speed)-speed2)*gamespeed div 64;
	    dely:=(random(speed)-speed2)*gamespeed div 64;
	  end
	  else
	  begin
	    theta:=cos(random*2*pi-pi);
	    delx:=round(speed*theta)*gamespeed div 64;
	    dely:=round(speed*(1-theta*theta))*gamespeed div 64;
	  end;
	end;
	if curves then
	begin
	  if random<changecurve then
	  begin
	    curvesin:=-curve2+random(curve);
	    curvecos:=round(sqrt(1-sqr(curvesin/32767))*32767);
	  end;
	  temp:=delx;
	  ltemp:=longint(delx)*curvecos-longint(dely)*curvesin;
	  if ltemp>0 then delx:=(ltemp+16384) div 32767
	   else delx:=(ltemp-16384) div 32767;
	  ltemp:=longint(dely)*curvecos+longint(temp)*curvesin;
	  if ltemp>0 then dely:=(ltemp+16384) div 32767
	   else dely:=(ltemp-16384) div 32767;
	end;
	inc(sx,delx);inc(sy,dely);
	oldx:=x;oldy:=y;
	x:=sx div 64;y:=sy div 64;
	xbr:=x+width-1;
	ybr:=y+height-1 ;
	if x<EnemyXMin then         {check bounds}
	begin
	  X:=EnemyXMin;delx:=abs(delx);
	end;
	if xbr>EnemyXMax then
	begin
	  X:=EnemyXMax-width;delx:=-abs(delx);
	end;
	if y<EnemyYMin then
	begin
	  y:=EnemyYMin;dely:=abs(dely);
	end;
	if ybr>EnemyYMax then
	begin
	  y:=EnemyYMax-height;dely:=-abs(dely);
	end;
	if (y<(PageHeight div 2 + 7)) and (ybr>(PageHeight div 2 -7)) then
	begin
	  if x<EnemyXMin+7 then delx:=abs(delx);
	  if xbr>EnemyXMax-7 then delx:=-abs(delx);
	end;  {checks for collision with enemy inlets}
	
      end;
    end;
  end;
  for i:=NumEnemies downto 1 do  {decrease lifetime counter}
  if enemy[i].supertime>0 then
  begin
    dec(enemy[i].supertime);
    if enemy[i].supertime<=0 then
    enemydestroyed(i,0,0,false);
  end;
end;        {MoveEnemies}


procedure UpdateMissiles;
var i,j:integer;
    OutOfBoundsX,OutOfBoundsY:boolean;
    BounceValue:integer;

begin
  BounceValue:=PowerUp[Bounce].value;

  asm   {ship's missiles}
	mov     cx,ship.nummissiles
	jcxz    @@finished
	lea     si,missiles
@@mloop:
	mov     ax,[si+missiletype.x]
	mov     [si+missiletype.oldx],ax
	mov     ax,[si+missiletype.y]
	mov     [si+missiletype.oldy],ax

	mov     ax,[si+missiletype.delx]
	add     ax,[si+missiletype.sx]
	mov     [si+missiletype.sx],ax
	shr     ax,6
	mov     [si+missiletype.x],ax
	add     ax,[ship.misswidth]
	mov     [si+missiletype.xbr],ax

	mov     ax,[si+missiletype.dely]
	add     ax,[si+missiletype.sy]
	mov     [si+missiletype.sy],ax
	shr     ax,6
	mov     [si+missiletype.y],ax
	add     ax,[ship.missheight]
	mov     [si+missiletype.ybr],ax

	inc     [si+missiletype.time]
	add     si,30
	dec     cx
	jnz     @@mloop
@@finished:
  end;

(*
  for i:=ship.NumMissiles downto 1 do
  with missiles[i] do
  begin
    if (x<10) or (xbr>PageWidth-10) or (y<10) or (ybr>PageHeight-10) or (time>MissileLife) then
    begin
      XPutMissPBM(oldx,oldy,VisiblePageOffs,oldbackgr^);
      missiletemp:=missiles[i];
      missiles[i]:=missiles[ship.nummissiles];  {move current missile to end of list}
      missiles[ship.NumMissiles]:=missiletemp;
      dec(ship.NumMissiles);                  {and delete}
	{need to swap because otherwise we lose the backgr pointers...}
	{actually we only need to swap the pointer to ship.nUmMissiles...}
    end;
  end;

*)

  asm
	mov     cx,ship.NumMissiles
	cmp     cx,0
	jz      @@Finished
	lea     si,missiles
	mov     ax,30
	mul     cx
	add     si,ax           {point si to end of missile record}
	mov     i,si            {save this value}
	sub     i,30            {i points to missiles[ship.NumMissiles]}
@@mloop2:
	sub     si,30           {si points to start of previous record}
	cmp     [si+missiletype.x],10
	jl      @@OutOfBoundsXLow
	cmp     [si+missiletype.xbr],(PageWidth-10)
	jg      @@OutOfBoundsXHigh
	cmp     [si+missiletype.y],10
	jl      @@OutOfBoundsYLow
	cmp     [si+missiletype.ybr],(PageHeight-10)
	jg      @@OutOfBoundsYHigh
	cmp     [si+missiletype.time],MissileLife
	jg      @@DeleteMissile
	jmp     @@DontDeleteMissile

@@OutOfBoundsXLow:
	mov     [si+missiletype.x],11
	jmp     @@OutOfBoundsX
@@OutOfBoundsXHigh:
	mov     [si+missiletype.x],(PageWidth-11)
@@OutOfBoundsX:
	cmp     BounceValue,0
	je      @@DeleteMissile
	neg     [si+missiletype.delx]        {bounce missile off wall}
	jmp     @@DontDeleteMissile

@@OutOfBoundsYLow:
	mov     [si+missiletype.y],11
	jmp     @@OutOfBoundsY
@@OutOfBoundsYHigh:
	mov     [si+missiletype.y],(PageHeight-11)
@@OutOfBoundsY:
	cmp     BounceValue,0
	je      @@DeleteMissile

	neg     [si+missiletype.dely]        {bounce missile off wall}
	jmp     @@DontDeleteMissile

@@DeleteMissile:
	push    cx
	push    si                      {store vars}
	push    [si+missiletype.oldx]
	push    [si+missiletype.oldy]
	push    VisiblePageOffs
	push    word ptr [si+missiletype.oldbackgr+2]
	push    word ptr [si+missiletype.oldbackgr]
	call    XPutMissPBM

	pop     si
	mov     bx,si                   {save si value in bx}

	mov     ax,ds
	mov     es,ax                   {point es to data segment}
	lea     di,missiletemp                  {es:di points to temp}
	mov     cx,15                   {no. of words to move}
	rep     movsw                   {temp:=missiles[i]}

	sub     si,30
	mov     di,si                   {missiles[i] now dest: es:di}
	mov     si,i                    {from missiles[ship.NumMissiles}
	mov     cx,15
	rep     movsw                   {missiles[i]:=missiles[Ship.NumMissiles]}

	mov     di,i                    {missile[ship.n..] dest}
	lea     si,missiletemp
	mov     cx,15
	rep     movsw                   {missiles[ship.n..]:=temp}

	dec     Ship.NumMissiles
	sub     i,30                    {point i to new missiles[ship.n...]}
	mov     si,bx
	pop     cx

@@DontDeleteMissile:
	dec     cx
	jnz     @@mloop2
@@Finished:
  end;

  {enemy missiles}

  asm   {enemy missiles}
	mov     cx,NumEnemyMissiles
	jcxz    @@finished
	lea     si,emissiles
@@mloop:
	mov     ax,[si+emissiletype.x]
	mov     [si+emissiletype.oldx],ax
	mov     ax,[si+emissiletype.y]
	mov     [si+emissiletype.oldy],ax

	mov     ax,[si+emissiletype.delx]
	add     ax,[si+emissiletype.sx]
	mov     [si+emissiletype.sx],ax
	shr     ax,6
	mov     [si+emissiletype.x],ax
	add     ax,[si+emissiletype.mtyp.width]
	mov     [si+emissiletype.xbr],ax

	mov     ax,[si+emissiletype.dely]
	add     ax,[si+emissiletype.sy]
	mov     [si+emissiletype.sy],ax
	shr     ax,6
	mov     [si+emissiletype.y],ax
	add     ax,[si+emissiletype.mtyp.height]
	mov     [si+emissiletype.ybr],ax

	add     si,SizeOfEmissileType
	dec     cx
	jnz     @@mloop
@@finished:
  end;


(*
  for i:=NumEnemyMissiles downto 1 do
  with emissiles[i] do
  with mtyp do
  begin
    OutOfBoundsX:=(x<10) or (xbr>PageWidth-10);
    OutOfBoundsY:=(y<10) or (ybr>PageHeight-10);
    if OutOfBoundsX or OutOfBoundsY then
    begin
      if mtyp.rebound then
      begin
	if OutOfBoundsX then delx:=-delx else dely:=-dely;
	PlaySound(boing);
      end
      else
      begin
	XPutMissPBM(oldx,oldy,VisiblePageOffs,oldbackgr^);
	emissiletemp:=emissiles[i];
	emissiles[i]:=emissiles[NumEnemyMissiles];    {move current missile}
	emissiles[NumEnemyMissiles]:=emissiletemp;    {to end of list}
	dec(NumEnemyMissiles);                        {and delete}
      end;
    end;
  end;
*)

  asm
	mov     cx,NumEnemyMissiles
	cmp     cx,0
	jz      @@Finished
	lea     si,emissiles
	mov     ax,SizeOfEmissileType
	mul     cx
	add     si,ax           {point si to end of missile record}
	mov     i,si            {save this value}
	sub     i,SizeOfEmissileType  {i points to emissiles[NumEnemyMissiles]}
@@emloop2:
	sub     si,SizeOfEmissileType   {si points to start of previous record}
	cmp     [si+emissiletype.x],10
	jl      @@OutOfBoundsX
	cmp     [si+emissiletype.xbr],(PageWidth-10)
	jg      @@OutOfBoundsX
	cmp     [si+emissiletype.y],10
	jl      @@OutOfBoundsY
	cmp     [si+emissiletype.ybr],(PageHeight-10)
	jg      @@OutOfBoundsY
	jmp     @@InBounds
@@OutOfBoundsX:
	cmp     [si+emissiletype.mtyp.rebound],1
	jl      @@RemoveMissile
	neg     [si+emissiletype.delx]
	jmp     @@BoingSound
@@OutOfBoundsY:
	cmp     [si+emissiletype.mtyp.rebound],1
	jl      @@RemoveMissile
	neg     [si+emissiletype.dely]
@@BoingSound:
	push    si
	push    cx
	push    word(boing)
	call    PlaySound;
	pop     cx
	pop     si
	jmp     @@InBounds
@@RemoveMissile:
	push    cx
	push    si                      {store vars}
	push    [si+emissiletype.oldx]
	push    [si+emissiletype.oldy]
	push    VisiblePageOffs
	push    word ptr [si+emissiletype.oldbackgr+2]
	push    word ptr [si+emissiletype.oldbackgr]
	call    XPutMissPBM

	pop     si
	mov     bx,si                   {it's going to change...}

	mov     ax,ds
	mov     es,ax                   {point es to date segment}
	lea     di,emissiletemp                 {es:di points to temp}
	mov     cx,(SizeOfEmissileType/2)     {no. of words to move}
	rep     movsw                   {temp:=missiles[i]}

	sub     si,SizeOfEmissileType
	mov     di,si                   {missiles[i] now dest: es:di}
	mov     si,i                    {from missiles[ship.NumMissiles}
	mov     cx,SizeOfEmissileType/2
	rep     movsw                   {missiles[i]:=missiles[Ship.NumMissiles]}

	mov     di,i                    {missile[ship.n..] dest}
	lea     si,emissiletemp
	mov     cx,SizeOfEmissileType/2
	rep     movsw                   {missiles[ship.n..]:=temp}

	dec     NumEnemyMissiles
	sub     i,SizeOfEmissileType    {point i to new missiles[ship.n...]}
	mov     si,bx
	pop     cx
@@InBounds:
	dec     cx
	jnz     @@emloop2
@@Finished:
  end;

end;    {UpdateMissiles}


procedure ObjectHit(i:integer);
var j:integer;
begin
  with objects do
  if pos[i].typ=crys then
  begin  {hit crystal}

{$IFDEF BONUSLEVEL}
    if pos[i].bonus then
    begin
      inc(BonusLevelCount);
      if GameInfo[Player].NumCrystals>1 then
      begin
	j:=0;
	repeat
	  inc(j);
	until (i<>j) and (objects.pos[j].typ=crys);
	with objects.pos[j] do
	begin
	  bonus:=true;
	  {draw bonus object}
	  XPutCBitmap(x,y,HiddenPageOffs,BCrystalPic^);
	end;
      end;
    end;
{$ENDIF}
    AddToScore(200);
    PlaySound(getcrystal);
    dec(GameInfo[Player].NumCrystals);
    ShowCrystals;
    if GameInfo[Player].NumCrystals=0 then
    if levels[GameInfo[Player].level].bonuslevel then
    with objects.pos[i] do
    begin
      LevelFinished:=true;
      XPutPBM(X,Y,VisiblePageOffs,backgr^);
    end
    else
    begin      {open gate to exit}
      XRectFill(Gate[Left].X+Gate[Left].Width,0,Gate[Right].X,11,VisiblePageOffs,0);
      XRectFill(Gate[Left].X+Gate[Left].Width,0,Gate[Right].X,11,HiddenPageOffs,0);
      PlaySound(gatesound);
    end;
  end
  else if pos[i].typ=mine then
  with pos[i] do
  begin
    ShipDestroyed:=true;
    AddEnemy(X,Y,1);
    PlaySound(explosn);
  end
  else
  begin
    inc(GameInfo[Player].NumSmartBombs);   {FIX: remove duplicate code here}
    ShowSmartBombs;
    PlaySound(allright);
    dec(GameInfo[Player].NumSmarts);
  end;
  with objects do
  with pos[i] do
  begin
    XPutPBM(X,Y,HiddenPageOffs,backgr^);
    if NumEnemyMines>0 then
      FixEMines(X,Y,Xbr,Ybr,HiddenPageOffs);
    delete:=true;
  end;    {remove pic}
end;


procedure CheckCollisions;
  {checks bounding box collisions}
var i,j:integer;
    HeavyFireValue:integer;
    SpecialGiven:boolean;

{Could make all routines assembler, but these seem to take little time
 so would probably not be worth it for now}

begin  {CheckCollisions}
  HeavyFireValue:=PowerUp[HeavyFire].value;
  SpecialGiven:=false;
  if not ShipDestroyed then
  begin
    for i:=NumEnemies downto 1 do   {ship vs enemies}
    with enemy[i] do
    with typ do
    if ntyp<>1 then {no collisions with explosions}
    begin
      if (xbr>=ship.x) and (ybr>=ship.y) and
	 (x<=ship.xbr) and (y<=ship.ybr) then
	 if ((ship.x<=x) and
	    CollideBitmaps(ship.mask[Ship.ShipDir],ship.height,ship.x,ship.y,mask[frame shr 8],height,x,y))
	   or ((x<ship.x) and
	    CollideBitmaps(mask[frame shr 8],height,x,y,ship.mask[ship.ShipDir],ship.height,ship.x,ship.y))
	 then
	 begin
	   if (ntyp>1) and not ShipDestroyed then
	   begin
	     ShipDestroyed:=true;
	     EnemyDestroyed(i,ship.delx,ship.dely,true);
	   end
	   else
	   if ntyp=0 then  {supercrystal}
	   begin
	     repeat
	       case random(22) of
		 0..2:with PowerUp[RapidFire] do
		      if value<=0 then
		      begin
			value:=value+random(TimeRan)+TimeMin;
			ShowPowerUps;
			SpecialGiven:=true;
		      end;
		 3..5:with PowerUp[MultiFire] do
		      if value<=0 then
		      begin
			value:=value+random(TimeRan)+TimeMin;
			ShowPowerUps;
			SpecialGiven:=true;
		      end;
		 6..8:with PowerUp[HeavyFire] do
		      if value<=0 then
		      begin
			value:=value+random(TimeRan)+TimeMin;
			ShowPowerUps;
			SpecialGiven:=true;
		      end;
		 9..11:with PowerUp[AssFire] do
		       if value<=0 then
		       begin
			 value:=value+random(TimeRan)+TimeMin;
			 ShowPowerUps;
			 SpecialGiven:=true;
		       end;
		 12..13:with PowerUp[AimedFire] do
			if value<=0 then
			begin
			  value:=value+random(TimeRan)+TimeMin;
			  ShowPowerUps;
			  SpecialGiven:=true;
			end;
		 14..15:with PowerUp[Bounce] do
			if value<=0 then
			begin
			  value:=value+random(TimeRan)+TimeMin;
			  ShowPowerUps;
			  SpecialGiven:=true;
			end;
		 16..17:if not NoMines then
			begin  {Destroy Mines}
			  if PowerUp[Shield].value=0 then PowerUp[Shield].value:=1;
			    {otherwise the ship is destroyed by the call to ObjectHit()}
			  for j:=1 to GameInfo[Player].NumObjects do
			    if objects.pos[j].typ=Mine then ObjectHit(j);
			  NoMines:=true;
					  {destroy all mines}
			  SmartBombed:=11;    {flash screen}
			  SpecialGiven:=true;
			end;
		 18:with PowerUp[Shield] do
		      begin
			value:=value+random(TimeRan)+TimeMin;
			ShowPowerUps;
			SpecialGiven:=true;
		      end;
		 19..22:if GateMoving then
			begin
			  GateMoving:=false;
			  with Gate[Left] do
			  begin
			    XPutPBM(X,0,VisiblePageOffs,Pic^);
			    XPutPBM(X,0,HiddenPageOffs,Pic^);
			  end;
			  with Gate[Right] do
			  begin
			    XPutPBM(X,0,VisiblePageOffs,Pic^);
			    XPutPBM(X,0,HiddenPageOffs,Pic^);
			  end;
			  SpecialGiven:=true;
			end;
	       end;
	     until SpecialGiven;
	     EnemyDestroyed(i,0,0,false);
	     PlaySound(ohyeah);
	   end;
	 end;
    end;

    for i:=1 to NumEnemyMines do      {ship vs. enemy mines}
    with emines do
    with emines.pos[i] do
    if ((xbr)>=ship.x) and ((ybr)>=ship.y) and
       (x<=(ship.xbr)) and (y<=(ship.ybr)) then
	 if ((ship.x<=x) and
	    CollideBitmaps(ship.mask[ship.ShipDir],ship.height,ship.x,ship.y,mask,height,x,y))
	   or ((x<ship.x) and
	    CollideBitmaps(emines.mask,height,x,y,ship.mask[ship.ShipDir],ship.height,ship.x,ship.y))
	 then ShipDestroyed:=true;


    for i:=GameInfo[Player].NumObjects downto 1 do      {ship vs objects}
    with objects do
    with pos[i] do
    if ((xbr)>=ship.x) and ((ybr)>=ship.y) and
       (x<=(ship.xbr)) and (y<=(ship.ybr)) then
	 if ((ship.x<=x) and
	    CollideBitmaps(ship.mask[ship.ShipDir],ship.height,ship.x,ship.y,mask[typ],height,x,y))
	   or ((x<ship.x) and
	    CollideBitmaps(mask[typ],height,x,y,ship.mask[Ship.ShipDir],ship.height,ship.x,ship.y))
	 then ObjectHit(i);
  end;

  for i:=1 to ship.NumMissiles do     {missiles vs enemies}
  for j:=NumEnemies downto 1 do
  with enemy[j] do
  if (ntyp<>1) then         {no explosions!}
  with typ do
  begin
    if ((missiles[i].xbr)>x) and (missiles[i].x<(xbr)) and
       ((missiles[i].ybr)>y) and (missiles[i].y<(ybr)) then
	 if ((missiles[i].x<=x) and
	   CollideBitmaps(ship.missmask,ship.missheight,missiles[i].x,missiles[i].y,
			  mask[frame shr 8],height,x,y))
	 or ((x<missiles[i].x) and
	   CollideBitmaps(mask[frame shr 8],height,x,y,
			  ship.missmask,ship.missheight,missiles[i].x,missiles[i].y))
	 then
	 begin
	   enemydestroyed(j,missiles[i].delx,missiles[i].dely,true);
	     {remove enemy}
	   missiles[i].time:=10000;  {remove missile}
	 end;
       end;


  for i:=NumEnemyMissiles downto 1 do   {ship vs enemy missiles}
  with emissiles[i] do
  with mtyp do
  begin
    if ((xbr)>=ship.x) and ((ybr)>=ship.y) and
       (x<=(ship.xbr)) and (y<=(ship.ybr)) then
	 if ((ship.x<=x) and
	    CollideBitmaps(ship.mask[ship.ShipDir],ship.height,ship.x,ship.y,mask,height,x,y))
	   or ((x<ship.x) and
	    CollideBitmaps(mask,height,x,y,ship.mask[ship.ShipDir],ship.height,ship.x,ship.y))
	 then
	 begin
	   ShipDestroyed:=true;
	       {Remove emissile..}
	   XPutMissPBM(oldx,oldy,VisiblePageOffs,oldbackgr^);
	   emissiletemp:=emissiles[i];
	   emissiles[i]:=emissiles[NumEnemyMissiles];
	     {move current missile to end of list}
	   emissiles[NumEnemyMissiles]:=emissiletemp;
	   dec(NumEnemyMissiles);
	       {EMissile removed}
	 end;
  end;


end;  {CheckCollisions}



procedure EraseSprites;  {erase old sprites on off-screen page}
var i,j,k:integer;
    tempobjpos:objpoint;
begin
  {erase old sprites}

  asm  {erase ship}
	push    ship.oldx
	push    ship.oldy
	push    HiddenPageOffs
	push    word ptr [ship.backgr+2]
	push    word ptr [ship.backgr]
	call    XPutPBM
@@finished:
  end;

  asm   {erase missiles}
	mov     cx,ship.NumMissiles
	jcxz    @@finished
	lea     si,missiles
	mov     ax,SizeOfMissileType
	mul     cx
	add     si,ax                   {si points to end of missiles record}
@@mloop:
	push    cx                      {store count}
	sub     si,SizeOfMissileType    {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+missiletype.oldx]          {push oldx}
	push    word ptr [si+missiletype.oldy]          {push oldy}
	push    HiddenPageOffs
	push    word ptr [si+missiletype.backgr+2]
	push    word ptr [si+missiletype.backgr]        {push backgr pointer}
	call    XPutMissPBM

	pop     si
	pop     cx                      {restore count}
	dec     cx
	jnz     @@mloop
@@finished:
  end;

  asm   {erase enemies}
	mov     cx,NumEnemies
	jcxz    @@Finished
	lea     si,enemy
	mov     ax,SizeOfEnemyType
	mul     cx
	add     si,ax                   {si points to end of enemy records}
@@eloop:
	push    cx                      {store count}
	sub     si,SizeOfEnemyType      {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+enemytype.oldx]    {push oldx}
	push    word ptr [si+enemytype.oldy]    {push oldy}
	push    HiddenPageOffs
	push    word ptr [si+enemytype.backgr+2]
	push    word ptr [si+enemytype.backgr]  {push backgr pointer}
	call    XPutPBM

	pop     si
	pop     cx                      {restore count}
	dec     cx
	jnz     @@eloop
@@finished:
  end;

  asm   {erase enemy missiles}
	mov     cx,NumEnemyMissiles
	jcxz    @@finished
	lea     si,emissiles
	mov     ax,SizeOfEmissileType
	mul     cx
	add     si,ax                   {si points to end of emissiles record}
@@emloop:
	push    cx                      {store count}
	sub     si,SizeOfEmissileType   {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+emissiletype.oldx]         {push x}
	push    word ptr [si+emissiletype.oldy]         {push y}
	push    HiddenPageOffs
	push    word ptr [si+emissiletype.backgr+2]
	push    word ptr [si+emissiletype.backgr]       {push backgr pointer}
	call    XPutMissPBM

	pop     si
	pop     cx                      {restore count}
	dec     cx
	jnz     @@emloop
@@finished:
  end;

  if GateMoving then {erase Gate}
  begin
    with Gate[Left] do
      XRectFill(X,0,X+Width,Height,HiddenPageOffs,0);
    with Gate[Right] do
      XRectFill(X,0,X+Width,Height,HiddenPageOffs,0);
  end;

  with TimeRecord do
  if ShowCountdown then
  XPutPBM(oldX,oldY,HiddenPageOffs,backgr^);


  for i:=GameInfo[Player].numobjects downto 1 do
  if objects.pos[i].delete then
  begin
    with objects do
    with pos[i] do
    begin
      XPutPBM(X,Y,HiddenPageOffs,backgr^);
      if NumEnemyMines>0 then
	FixEMines(X,Y,Xbr,Ybr,HiddenPageOffs);
    end;
    tempobjpos:=objects.pos[i];
    objects.pos[i]:=objects.pos[GameInfo[Player].NumObjects];
    objects.pos[GameInfo[Player].NumObjects]:=tempobjpos;
	{swapped with end of list and removed}
    dec(GameInfo[Player].NumObjects);

{$IFDEF BONUSLEVEL}
    with GameInfo[Player] do
    if levels[level].bonuslevel then
    begin
      j:=0;
      repeat
	inc(j)
      until (j>NumObjects) or objects.pos[j].bonus;
      if j<=NumObjects then
      begin
	with objects.pos[j] do
	XPutCBitmap(x,y,HiddenpageOffs,BCrystalpic^);
	{put picture for crystal j}
      end;
    end;
{$ENDIF}
  end;

  if emines.adding then
  with emines.pos[NumEnemyMines] do
  begin
    XPutCBitmap(x,y,HiddenPageOffs,emines.pic^);
    emines.adding:=false;
  end;

end;  {EraseSprites}

{$IFDEF DEBUG}
{$F+}   {Dummy procedures for speed testing}
procedure DXGetPBM(x,y,bmwidth,height,page:word;var g);
begin
end;

Procedure DXPutCBitmap(x,y,page:word;var g);
begin
end;
{$F-}
{$ENDIF DEBUG}

procedure DisplayTime;
type timestr=string[8];
var i,mins,bits:longint;
    secs:longint;
    s,temp:timestr;
    p:pointer;

  procedure str2(i:longint;var s:timestr);
  begin
    str(i:2,s);
    if s[1]=' ' then s[1]:='0';
  end;

begin
  if not ShowCountDown then exit;
  with TimeRecord do
  begin
    with GameInfo[Player] do
    secs:=levels[level].time*FrameRate-TimeOnLevel;
    if secs<0 then exit;
    bits:=Secs mod FrameRate;
    bits:=trunc(bits*100/FrameRate);
    secs:=secs div FrameRate;
    mins:=secs div 60;
    secs:=secs mod 60;
    str2(bits,s);
    if (mins>0) or (secs>0) then
    begin
      if mins>0 then str2(secs,temp) else str(secs,temp);
      s:=temp+':'+s;
      if mins>0 then
      begin
	str(mins,temp);
	s:=temp+':'+s;
      end;
    end;
{$IFDEF DEBUGDEMO}
    str(demoptr,s);
{$ENDIF}
    while length(s)<7 do s:=' '+s;
    for i:=0 to 6 do
      if s[i+1] in ['0'..'9'] then
	XPutCBitmap(X+i*5,Y,HiddenPageOffs,SmallFont.pic[s[i+1]]^);
  end;
end;



procedure DrawSprites;
var temp:pointer;
begin

  {Get backgrounds}

  asm    {timer}
	cmp     ShowCountDown,1         {are we showing the timer?}
	jl      @@finished
	mov     ax,TimeRecord.X
	mov     TimeRecord.OldX,ax
	mov     ax,TimeRecord.Y
	mov     TimeRecord.OldY,ax
	mov     ax,VisiblePageX
	add     ax,TimerX
	cmp     ax,(PageWidth-46)       {prevents the right edge of the timer}
	jle     @@XOK                   {overlapping the screen border}
	mov     ax,(PageWidth-46)
@@XOK:
	mov     TimeRecord.X,ax
	mov     ax,VisiblePageY
	add     ax,TimerY
	mov     TimeRecord.Y,ax

	push    TimeRecord.X
	push    TimeRecord.Y
	push    word(9)                 {naughty, but...}
	push    SmallFont.Height
	push    HiddenPageOffs
	push    word ptr [TimeRecord.backgr+2]
	push    word ptr [TimeRecord.backgr]
	call    XGetPBM

	mov     ax,word ptr [TimeRecord.backgr]
	xchg    ax,word ptr [TimeRecord.oldbackgr]
	mov     word ptr [TimeRecord.backgr],ax {exchange backgr and oldbackgr}
	mov     ax,word ptr [TimeRecord.backgr+2]
	xchg    ax,word ptr [TimeRecord.oldbackgr+2]
	mov     word ptr [TimeRecord.backgr+2],ax
@@finished:
  end;

  asm {ship}
	cmp     ship.y,0
	js      @@finished
	push    ship.x
	push    ship.y
	push    ship.bmwidth
	push    ship.height
	push    HiddenPageOffs
	push    word ptr [ship.backgr+2]
	push    word ptr [ship.backgr]
	call    XGetPBM

	mov     ax,word ptr [ship.backgr]
	xchg    ax,word ptr [ship.oldbackgr]
	mov     word ptr [ship.backgr],ax       {exchange backgr and oldbackgr}
	mov     ax,word ptr [ship.backgr+2]
	xchg    ax,word ptr [ship.oldbackgr+2]
	mov     word ptr [ship.backgr+2],ax
@@finished:
  end;

  asm   {missiles}
	mov     cx,ship.NumMissiles
	jcxz    @@finished
	lea     si,missiles
	mov     ax,SizeOfMissileType
	mul     cx
	add     si,ax                   {si points to end of missiles record}
@@mloop:
	push    cx                      {store count}
	sub     si,SizeOfMissileType    {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+missiletype.x]     {push x}
	push    word ptr [si+missiletype.y]     {push y}
	push    ship.MissHeight
	push    HiddenPageOffs
	push    word ptr [si+missiletype.backgr+2]
	push    word ptr [si+missiletype.backgr]        {push backgr pointer}
	call    XGetMissPBM

	pop     si
	pop     cx                      {restore count}

	mov     ax,word ptr [si+missiletype.backgr]
	xchg    ax,word ptr [si+missiletype.oldbackgr]
	mov     word ptr [si+missiletype.backgr],ax
	mov     ax,word ptr [si+missiletype.backgr+2]
	xchg    ax,word ptr [si+missiletype.oldbackgr+2]
	mov     word ptr [si+missiletype.backgr+2],ax

	dec     cx
	jnz     @@mloop
@@finished:
  end;


  asm   {enemies}
	mov     cx,NumEnemies
	jcxz    @@Finished
	lea     si,enemy
	mov     ax,SizeOfEnemyType
	mul     cx
	add     si,ax                   {si points to end of enemy records}
@@eloop:
	push    cx                      {store count}
	sub     si,SizeOfEnemyType      {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+enemytype.x]       {push x}
	push    word ptr [si+enemytype.y]       {push y}
	push    word ptr [si+enemytype.typ.bmwidth]
	push    word ptr [si+enemytype.typ.height]
	push    HiddenPageOffs
	push    word ptr [si+enemytype.backgr+2]
	push    word ptr [si+enemytype.backgr]  {push backgr pointer}
	call    XGetPBM

	pop     si
	pop     cx                      {restore count}

	mov     ax,word ptr [si+enemytype.backgr]
	xchg    ax,word ptr [si+enemytype.oldbackgr]
	mov     word ptr [si+enemytype.backgr],ax
	mov     ax,word ptr [si+enemytype.backgr+2]
	xchg    ax,word ptr [si+enemytype.oldbackgr+2]
	mov     word ptr [si+enemytype.backgr+2],ax

	dec     cx
	jnz     @@eloop
@@finished:
  end;

  asm   {enemy missiles}
	mov     cx,NumEnemyMissiles
	jcxz    @@finished
	lea     si,emissiles
	mov     ax,SizeOfEmissileType
	mul     cx
	add     si,ax                   {si points to end of emissiles record}
@@exloop:
	push    cx                      {store count}
	sub     si,SizeOfEmissileType   {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+emissiletype.x]    {push x}
	push    word ptr [si+emissiletype.y]    {push y}
	push    word ptr [si+emissiletype.mtyp.height]
	push    HiddenPageOffs
	push    word ptr [si+emissiletype.backgr+2]
	push    word ptr [si+emissiletype.backgr]       {push backgr pointer}
	call    XGetMissPBM

	pop     si
	pop     cx                      {restore count}

	mov     ax,word ptr [si+emissiletype.backgr]
	xchg    ax,word ptr [si+emissiletype.oldbackgr]
	mov     word ptr [si+emissiletype.backgr],ax
	mov     ax,word ptr [si+emissiletype.backgr+2]
	xchg    ax,word ptr [si+emissiletype.oldbackgr+2]
	mov     word ptr [si+emissiletype.backgr+2],ax

	dec     cx
	jnz     @@exloop
@@finished:
  end;

  {display new sprites}


  if GateMoving then
  begin
    with Gate[Left] do
      XPutPBM(X,0,HiddenPageOffs,Pic^);
    with Gate[Right] do
      XPutPBM(X,0,HiddenPageOffs,Pic^);
  end;


  asm   {ship}
	cmp     ship.y,0
	js      @@finished              {Has the ship left the level?}
	cmp     ShipDestroyedCount,StartShipDestroyedCount-2
	jle     @@finished              {don't display if destroyed}
	mov     bx,[Ship.ShipDir]
	shl     bx,2                    {offset into ship.pic array}
	push    ship.x
	push    ship.y
	push    HiddenPageOffs
	push    word ptr [ship.pic+bx+2]
	push    word ptr [ship.pic+bx]
	call    XPutCBitmap             {put ship bitmap}
@@finished:
  end;


  asm   {missiles}
	mov     cx,ship.NumMissiles
	jcxz    @@finished
	lea     si,missiles
	mov     ax,SizeOfMissileType
	mul     cx
	add     si,ax                   {si points to end of missiles record}
@@mloop:
	push    cx                      {store count}
	sub     si,SizeOfMissileType    {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+missiletype.x]     {push x}
	push    word ptr [si+missiletype.y]     {push y}
	push    HiddenPageOffs
	push    word ptr [ship.misspic+2]
	push    word ptr [ship.misspic]         {push pic pointer}
	call    XPutCBitmap

	pop     si
	pop     cx                      {restore count}
	dec     cx
	jnz     @@mloop
@@finished:
  end;


  asm   {enemies}
	mov     cx,NumEnemies
	jcxz    @@finished
	lea     si,enemy
	mov     ax,SizeOfEnemyType
	mul     cx
	add     si,ax                   {si points to end of missiles record}
@@eloop:
	push    cx                      {store count}
	sub     si,SizeOfEnemyType      {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+enemytype.x]       {push x}
	push    word ptr [si+enemytype.y]       {push y}
	push    HiddenPageOffs
	mov     bx,word ptr [si+enemytype.frame]
	shr     bx,8                            {frame shr 8: low byte}
	shl     bx,2                            {doublewords}
	add     si,enemytype.typ.pic            {point si to pic list}
	push    word ptr [si+bx+2]
	push    word ptr [si+bx]                {push pic pointer}
	call    XPutCBitmap

	pop     si
	pop     cx                      {restore count}
	dec     cx
	jnz     @@eloop
@@finished:
  end;

  asm   {enemy missiles}
	mov     cx,NumEnemyMissiles
	jcxz    @@finished
	lea     si,emissiles
	mov     ax,SizeOfEmissileType
	mul     cx
	add     si,ax                   {si points to end of emissiles record}
@@emloop:
	push    cx                      {store count}
	sub     si,SizeOfEmissileType   {point to previous record}
	push    si                      {store pointer}
	push    word ptr [si+emissiletype.x]    {push x}
	push    word ptr [si+emissiletype.y]    {push y}
	push    HiddenPageOffs
	push    word ptr [si+emissiletype.mtyp.pic+2]
	push    word ptr [si+emissiletype.mtyp.pic]     {push pic pointer}
	call    XPutCBitmap

	pop     si
	pop     cx                      {restore count}
	dec     cx
	jnz     @@emloop
@@finished:
  end;


  DisplayTime;

end;    {DrawSprites}

{$IFDEF DEBUG}
procedure Testspeed;  {Debugging routine}
var i,count:longint;
begin
  count:=0;timecount:=0;
  for i:=1 to NumEnemyMissiles do
  begin
    emissiles[i].delx:=0;
    emissiles[i].dely:=0;
  end;
(*  SoundBlasterInitialised:=false;*)
  write(^g);
  repeat                           {all times for 70 objects}

(***NO SOUND***)

(*    EraseSprites;                  5.7/frame with 13 enemies, 51 missiles
				   4.3/frame      15    "     87    "
				   5.2            15          72     19 shipmiss (new)
    MoveEnemies;                   26/frame with 16 enemies
    UpdateMissiles;                91/frame for 88 emissiles
    CheckCollisions;               ~46/frame with 15 enemies, 0 missiles
				   ~30/frame with 16 enemies, 21 missiles
				   ~20/frame with 15 enemies,18 missiles,68 enemy missiles
    MoveShip;                      135/frame
				   {110/frame with Ticks=12000}
    DrawSprites;                   2.5/frame with 15 enemies,62 emissiles
			      2.4/frame with 13 enemies,55 emissiles, 24 missiles
			      2.9            13         65            12 (new)
*)

(***WITH SOUND***)

(*  DrawSprites;
    PlaySound(fire4);                2.1/frame with 16 enemies, 79 missiles

    MoveShip;
    PlaySound(fire4);              109/frame
*)

(*    XPageFlip(VisiblePageX,VisiblePageY);  (* 59.6/sec  *)
(*    FixedXPageFlip(VisiblePageX,VisiblePageY);  (* 59.6/sec *)
(*    PlaySound(random(20)+1);                 (* 600/frame (?) *)

(*    if random<0.00001 then playsound(random(20)+1);
       (* 152000/s with sound, 156000/s without sound *)
    MoveShip;
    inc(count);
  until timecount>=Ticks*10; {10 sec, period =6554}  {66.6 frames/sec}
{  Soundblasterinitialised:=true;}
  XTextMode;
  writeln('iterations: ',count,'   iters/frame: ',(count div 66)/10:5:1,'   enemies: ',
	   numenemies,'   missiles: ',numenemymissiles);
  writeln('ship missiles: ',ship.nummissiles,' emines: ',NumEnemyMines,' objects: ',GameInfo[Player].NumObjects,' ',ch);
  ch:=readkey;halt;
end;

{$ENDIF DEBUG}

procedure NewLife;
var i:integer;
    PU:PowerUpType;
    temp:longint;
begin
  dec(GameInfo[Player].Lives);
  if GameMode=TwoPlayer then
  begin
    if Player=Player1 then
    begin
      if GameInfo[Player2].Lives>0  then
      begin
	Player:=Player2;
	SwapObjects;
      end
    end
    else if GameInfo[Player1].Lives>0 then
    begin
      Player:=Player1;
      SwapObjects;
    end;
  end;
  with GameInfo[Player] do
  with PlayerInfo[Player] do
    GameSpeed:=round(BaseGameSpeed* (DiffInfo[DiffLevel].speedfactor+GameClockedSpeedUp[GameClocked]));

  if (GameInfo[Player1].Lives=0) and
     ((GameMode=OnePlayer) or (GameInfo[Player2].Lives=0))
     then GameOver:=true
  else
  begin
    for PU:=Shield to Bounce do
    with powerUp[PU] do
    begin
      value:=0;position:=0;
    end;
    ShipDestroyedCount:=StartShipDestroyedCount;
    StartNewLevel;
    if GameMode=TwoPlayer then
    begin
      TextWindow(PlayerStr[Player]+' READY');
    end
    else TextWindow('READY');
    if demomode then mydelay(300) else
      repeat until keypressed or (ButtonPressed<>0);
    ClearInputBuffers(false);
    RemoveTextWindow;
    mmotion(i,i);
  end;
end;   {NewLife}

procedure PauseGame(message:string);
var i:integer;
begin
  GameInfo[Player].Timing:=false;
  PollOnTimer:=true;
  if maxavail<(102*40+2) then
  begin
    XTextMode;
    writeln('Error: Insufficient memory to allocate message window.');
    halt(0);
  end;
  TextWindow(message);
  WaitForEvent(NoStars);
  RemoveTextWindow;
  GameInfo[Player].Timing:=true;
  PollOnTimer:=false;
  mmotion(i,i);
end;

procedure ClearEnemies;
var i:integer;
begin
  for i:=NumEnemies downto 1 do DeleteEnemy(i);
      {delete enemies, regain allocated background memory}
end;

procedure LevelOver;
begin

{$IFDEF BONUSLEVEL}
  if not levels[GameInfo[Player].level].BonusLevel then
  begin
    PlaySound(phew);
    GiveBonus;
  end else
  begin
    GiveBonusLevelBonus;
  end;
{$ELSE}
  PlaySound(phew);
  GiveBonus;
{$ENDIF}
  with GameInfo[Player] do
  begin
    inc(level);
    inc(totallevel);                        {counts total # levels}
    if level>maxlevel then                  {level counter stops}
    begin
      if GameClocked<MaxGameClocked then inc(GameClocked);
      ShowGameClockedMessage;
      level:=1;
    end;
    SetUpNewLevel(Player);
  end;
  StartNewLevel;
end;

procedure Game;
begin
  StarsFinished;        {free memory used for starfield}
  InitialiseVariables;
  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page0Offs,0);
  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page1Offs,0);
			{clear screen}
  SetXMode;
  XPutPalStruc(palette);
  if demomode or recording then randseed:=SavedRandSeed;
  demoptr:=0;
  SetupNewLevel(Player1);
  if GameMode=TwoPlayer then
  begin
    SwapObjects;
    SetupNewLevel(Player2);
    SwapObjects;
  end;
  StartNewLevel;
  if GameMode=TwoPlayer then
  begin
    TextWindow(PlayerStr[Player]+' READY');
  end
  else TextWindow('READY');
  if demomode then mydelay(300) else WaitForEvent(NoStars);
  RemoveTextWindow;

  repeat
    PollOnTimer:=false;  {turn off timer SB polling: poll manually}
    GameInfo[Player].timing:=true;
    FrameCount:=0;
    repeat

(*      if ch='`' then
      begin
    ZTimerOn;

    for i:=1 to 100 do
    begin
	asm  {erase ship}
	      push      ship.oldx
	      push      ship.oldy
	      push      HiddenPageOffs
	      push      word ptr [ship.backgr+2]
	      push      word ptr [ship.backgr]
	      call      XPutPBM
      @@finished:
	end;
    end;
	ZTimerOff;
	TimerReport('Erase Ship*1000');

	ZTimerOn;
	for i:=1 to 100 do
     begin
	asm {ship}
	  cmp   ship.y,0
	  js    @@finished
	  push  ship.x
	  push  ship.y
	  push  ship.bmwidth
	  push  ship.height
	  push  HiddenPageOffs
	  push  word ptr [ship.backgr+2]
	  push  word ptr [ship.backgr]
	  call  XGetPBM

	  mov   ax,word ptr [ship.backgr]
	  xchg  ax,word ptr [ship.oldbackgr]
	  mov   word ptr [ship.backgr],ax       {exchange backgr and oldbackgr}
	  mov   ax,word ptr [ship.backgr+2]
	  xchg  ax,word ptr [ship.oldbackgr+2]
	  mov   word ptr [ship.backgr+2],ax
  @@finished:
    end;
  end;
    ZTimerOff;
    TimerReport('Get Background');


	PrintTimes;
      end;
*)
      EraseSprites;
      MoveEnemies;
      UpdateMissiles;
      CheckCollisions;
{$IFDEF ZTIMER}
      if (ch='`') or (ch='1') then ZTimerOn;
{$ENDIF}
      if SoundBlasterInitialised then SBPoll;    {manually update sounds}
{$IFDEF ZTIMER}
      if (ch='`') or (ch='1') then begin ZTimerOff;TimerReport('SbPoll');end;
{$ENDIF}

      if ShipDestroyed then
      begin
	if (PowerUp[Shield].value>0) then ShipDestroyed:=false
	else
	begin
	  with ship do begin oldx:=x;oldy:=y;end;
	  if ShipDestroyedCount=(StartShipDestroyedCount-1) then
	  begin
	    PlaySound(explosn);
	    AddEnemy(Ship.X,Ship.Y,1);   {add enemy after ship has been erased}
	  end;
	  dec(ShipDestroyedCount);
	end;
      end;

      if not ShipDestroyed then MoveShip;

      if (SmartBombed>1) and (FrameCount and 1 =0) then
      begin
	dec(SmartBombed);
	with SmartBombPal[SmartBombed] do XSetRGB(0,R,G,B);
		    {fade screen flash}
      end;

      DrawSprites;

      if GateMoving then
      begin
	inc(GateMoveCount,levels[GameInfo[Player].level].GateMove);
	  {fixed point 2:6}
	if (GateMoveCount>63) then MoveGate;
      end;


      XPageFlip(VisiblePageX,VisiblePageY);
      inc(FrameCount);
      if GameInfo[Player].timing then inc(GameInfo[Player].TimeOnLevel);
{$IFDEF ZTIMER}
      if ch='`' then PrintTimes;
{$ENDIF}
      if keypressed then
      begin
        if numkeypresses(kESC)>0 then
        begin
	  if (not recording) then
	  begin
            ClearInputBuffers(NoStars);
	    PauseGame('QUIT?');
	    GameOver:=upcase(LastKeyHit)='Y';
	    if GameOver then GameInfo[Player].Score:=0;  {Don't give a high score}
	  end;
        end;
        if numkeypresses(kP)>0 then
        begin
	  PauseGame('PAUSE');
	end;
        if (keys[kEQUAL]) or (keys[kKEYPADPLUS]) then
	begin
	  inc(SoundVolume,4);
	  if SoundVolume>MaxSoundVolume then SoundVolume:=MaxSoundVolume;
	  SetSBVolume(SoundVolume)
	end;
        if (keys[kMINUS]) or (keys[kKEYPADMINUS]) then
        begin
	  dec(SoundVolume,4);
	  if SoundVolume<0 then SoundVolume:=0;
	  SetSBVolume(SoundVolume)
        end;
        if numkeypresses(kS)>0 then SoundsOn:=not SoundsOn;

{$IFDEF DEBUG}
        if numkeypresses(kT)>0 then TestSpeed;
{$ENDIF}
{$IFDEF CHEATS}
        if numkeypresses(kQ)>0 then
        begin
	  PowerUp[RapidFire].value:=random(2000);
	  PowerUp[HeavyFire].value:=random(2000)+2000;
	  PowerUp[MultiFire].value:=random(2000);
{                 PowerUp[AimedFire].value:=random(2000);}
	  PowerUp[AssFire].value:=random(2000);
	  PowerUp[Shield].value:=random(2000);
	  PowerUp[Bounce].value:=random(2000);
	  ShowPowerUps;
	end;
        if numkeypresses(kW)>0 then
        begin
	  PowerUp[AimedFire].value:=random(2000);
	  ShowPowerUps;
	end;
{$ENDIF}
      end;
      with GameInfo[Player] do
      if TimeOnLevel>levels[level].time * FrameRate then
      begin
	if levels[GameInfo[Player].level].BonusLevel then LevelFinished:=true;
	with PlayerInfo[Player] do
	   GameSpeed:=round(BaseGameSpeed* (DiffInfo[DiffLevel].speedfactor+GameClockedSpeedUp[GameClocked]))
	    +(TimeOnLevel-(levels[level].time * FrameRate))
	     *BaseGameSpeed div (longint(FrameRate)*32);
	    {increase by BaseGameSpeed every 32 secs}
      end;

    until LevelFinished or (ShipDestroyedCount=0) or GameOver;
    PollOnTimer:=true;   {polling SB on timer interrupt now}
    GameInfo[Player].Timing:=false;
    ClearEnemies;        {restore memory for enemy backgrounds}
    if LevelFinished then LevelOver
      else if ShipDestroyed then NewLife;
  until GameOver;
  EndScreen;
end;

procedure SoundOptionsMenu;
var j:integer;
    ts:shortstring;
    tmpSBAddr,tmpSBIrq,tmpSBDMA:word;
    tmpSoundCard:integer;
const Menu3:MenuListType=(num:6;s:((name:'No soundcard';mtype:item),
				   (name:'Sound Volume';mtype:slidebar;min:0;max:128;value:96),
				   (name:'Port:';mtype:item),
				   (name:'IRQ:';mtype:item),
				   (name:'DMA';mtype:item),
				   (name:'Maximum Sounds:';mtype:item),
                                   (name:'';mtype:item)));

begin
  j:=0;
  tmpSBAddr:=SBAddr;
  tmpSBIrq:=SBIrq;
  tmpSBDMA:=SBDMA;
  tmpSoundCard:=SoundCard;
  repeat
    if tmpSoundCard=SoundBlaster then
    begin
      Menu3.num:=6;
      Menu3.s[0].name:='Sound Blaster'
    end
    else
    begin
      Menu3.s[0].name:='No Soundcard';
      Menu3.num:=1;
    end;
    Menu3.s[1].value:=SoundVolume;
    strhex(tmpSBAddr,ts);
    Menu3.s[2].name:='Port: '+ts;
    str(tmpSBIrq,ts);
    Menu3.s[3].name:='IRQ: '+ts;
    str(tmpSBDMA,ts);
    Menu3.s[4].name:='DMA: '+ts;
    str(MaxSoundEffects,ts);
    Menu3.s[5].name:='Max. Sounds: '+ts;
    j:=XMenu(Menu3,j,60);
    case j of
      0:begin
	  if tmpSoundCard=SoundBlaster then tmpSoundCard:=NoSoundCard
	  else tmpSoundCard:=SoundBlaster;
	end;
      2:begin
	  tmpSBAddr:=tmpSBAddr+16;
	  if tmpSBAddr>$280 then tmpSBAddr:=$210;
	end;
      3:begin
	  inc(tmpSBIrq);
	  if tmpSBIrq>15 then tmpSBIrq:=2;
	end;
      4:begin
	  inc(tmpSBDMA);
	  if tmpSBDMA>3 then tmpSBDMA:=0;
	end;
      5:begin
	  inc(MaxSoundEffects);
	  if MaxSoundEffects>CompiledMaximumSoundEffects then
	     MaxSoundEffects:=1;
	end;
    end;
    SoundVolume:=Menu3.s[1].value;
  until j=-1;
  if SoundCard=SoundBlaster then
    SetSbVolume(SoundVolume); {a safe procedure even if the card does not exist}
{$IFDEF GUS}
  if GUSPresent then
    UltraSetLinearVolume(i,SoundVolume shl 3);  {range 0..511}
{$ENDIF}
  if SoundBlasterInitialised then SBDone; {close down SB}
  SBAddr:=tmpSBAddr;SBIRQ:=tmpSBIrq;SBDMA:=tmpSBDMA;
  if tmpSoundCard=SoundBlaster then
  begin
    SoundCard:=SoundBlaster;
    if SBInit(false)=0 then
    begin
      if not SoundsLoaded then
      begin
	if memavail<250000 then
	begin
	  XPrintfCenterStars(160,220,VisiblePageOffs,BaseColor,'Insufficient memory to load sounds.');
	  repeat StarFieldStep until ButtonPressed<>0;
	  SoundCard:=NoSoundCard;
	  SBDone;
	end else InitialiseSounds;
      end;
    end
    else
    begin {error initialising SB}
      XPrintfCenterStars(160,220,VisiblePageOffs,BaseColor,'Error initialising SoundBlaster');
      repeat StarFieldStep until ButtonPressed<>0;
      SoundCard:=NoSoundCard;
    end;
  end else SoundCard:=NoSoundCard;
end;


procedure OptionsMenu;
var j:integer;
    Control:InputDeviceType;
const Menu3:MenuListType=(num:7;s:((name:'One Player';mtype:item),
				   (name:'Sound Setup';mtype:item),
				   (name:'Horizontal Sensitivity';mtype:slidebar;min:4;max:256;value:64),
				   (name:' Vertical Sensitivity ';mtype:slidebar;min:4;max:256;value:64),
				   (name:'Difficulty:';mtype:item),
				   (name:'Input:';mtype:item),
				   (name:'Input Device Setup';mtype:item)));


const ControlOption:array[MouseInput..KeyboardInput] of string[30]=
 ('Input Device: Mouse','Input Device: Joystick','Input Device: Keyboard');

begin
  j:=0;
  repeat
    Menu3.s[2].value:=PlayerInfo[Player].HInputSpeed;
    Menu3.s[3].value:=PlayerInfo[Player].VInputSpeed;
    Menu3.s[4].name:='Difficulty: '+DiffName[PlayerInfo[Player].DiffLevel];
    if GameMode=OnePlayer then
      Menu3.s[0].name:='One Player' else
      if Player=Player1 then Menu3.s[0].name:='Two Players: Player One'
	else Menu3.s[0].name:='Two Players: Player Two';
    Control:=PlayerInfo[Player].InputDevice;
    Menu3.s[5].name:=ControlOption[Control];
    j:=XMenu(Menu3,j,60);
    InitJoyUnit;
    case j of
      1:SoundOptionsMenu;
      4:with PlayerInfo[Player] do
	begin
	  DiffLevel:=(Difflevel+1) mod MaxDiffLevel;
	end;
      5:with PlayerInfo[Player] do
	begin
	  if (Control=KeyboardInput) then
          begin
	    if (not MousePresent) then Control:=JoyInput
	      else Control:=MouseInput;
          end else inc(Control);
	  if (not JoyAPresent) and (Control=JoyInput) then
            Control:=KeyboardInput;
	end;
      6:begin
          ResetStars(20,80,320-20,230);
          XRectFill(20,80,320-20,230,VisiblePageOffs,0);
          case Control of
            MouseInput:    InitialiseMouse(130);
            JoyInput:      InitialiseJoyStick(130);
            KeyboardInput: InitialiseKeyboard(130);
          end;
        end;
      0:begin
	  PlayerInfo[Player].HInputSpeed:=Menu3.s[2].value;
	  PlayerInfo[Player].VInputSpeed:=Menu3.s[3].value;
	  PlayerInfo[Player].InputDevice:=Control;
	  if GameMode=OnePlayer then GameMode:=TwoPlayer
	  else if Player=Player1 then Player:=Player2
	    else begin
		   GameMode:=OnePlayer;
		   Player:=Player1;
		 end;
	end;
    end;
    if j<>0 then
    begin
      PlayerInfo[Player].HInputSpeed:=Menu3.s[2].value;
      PlayerInfo[Player].VInputSpeed:=Menu3.s[3].value;
      PlayerInfo[Player].InputDevice:=Control;

    end;
  until j=-1;
  with JoyCal[1] do
    if PlayerInfo[Player].InputDevice=JoyInput then
      if (not JoyStickACalibrated)
      or (XMin>XCentreMin) or (XCentreMin>XCentreMax)
      or (XCentreMax>XMax) or (YMin>YCentreMin)
      or (YCentreMin>YCentreMax) or (YCentreMax>YMax)
      then InitialiseJoyStick(220);
	{make sure joystick params are reasonable}
{$IFDEF GUS}
  if GUSPresent then
    UltraSetLinearVolume(i,SoundVolume shl 3);  {range 0..511}
{$ENDIF}
  Player:=Player1;
  WriteDefaults;
end;

procedure DemoMenu;
var  dir:dirstr;name:namestr;ext:extstr;
     j:integer;
const Menu2:MenuListType=(num:2;s:((name:'Play Demo';mtype:item),
				   (name:'Record Demo';mtype:item),
				   (name:'';mtype:item),
				   (name:'';mtype:item),
				   (name:'';mtype:item),
				   (name:'';mtype:item),
                                   (name:'';mtype:item)));

begin
  j:=XMenu(Menu2,0,60);
  case j of
    0:begin
	XPrintfCenterStars(110,220,VisiblePageOffs,BaseColor,'Demo file name: ');
	demofilename:=GetFileName(170,220,VisiblePageOffs,BaseColor);
	if (demofilename<>'') then
	begin
	  if (demofilename=#27) or (not SetDemoFile(demofilename)) then
	  begin
	    if (demofilename=#27) then
	    XPrintfCenterStars(160,220,VisiblePageOffs,BaseColor,'        No demo files found.        ')
	    else
	    XPrintfCenterStars(160,220,VisiblePageOffs,BaseColor,'       Cannot open file '''
				      +demofilename+'''        ');
	    demofilename:='xquest.dmo';
	    SetDemoFile(demofilename);
            WaitForEvent(Stars);
	  end
	  else
	  begin
	    SavedPlayerInfo:=PlayerInfo;
	    PlayerInfo:=DemoPlayerInfo;
	    demomode:=true;
  {$IFDEF DEBUG}
	    writeln(debugfile);
	    writeln(debugfile,'Playback of file ',demofilename);
  {$ENDIF}
	    Game;
	    PlayerInfo:=SavedPlayerInfo;
	    GameMode:=SavedGameMode;
	  end;
	end;
      end;
    1:begin
	XPrintfCenterStars(110,220,VisiblePageOffs,BaseColor,'Demo file name: ');
	demofilename:=GetString(170,220,8,120,VisiblePageOffs,BaseColor);
	fsplit(demofilename,dir,name,ext);
	demofilename:=dir+name+'.dmo';
	if exist(demofilename) then
	begin
	  XPrintfCenterStars(160,220,VisiblePageOffs,BaseColor,'File '''+demofilename+''' exists. Overwrite? (y/n) ');
	  if YesNoStars('N')='N' then demofilename:='.dmo';
	end;
	if demofilename<>'.dmo' then
	begin
	  {$I-}
	  close(demofile);
	  {$I+}
	  if ioresult<>0 then begin end;   {ignore error here: clear ioresult}
	  {I-}
	  assign(demofile,demofilename);
	  if ioresult<>0 then halt;
	  rewrite(demofile,1);
	  randomize;
	  SavedRandSeed:=randseed;
	  blockwrite(demofile,SavedRandSeed,4);
	  blockwrite(demofile,GameMode,sizeof(GameModeType));
	  blockwrite(demofile,PlayerInfo,sizeof(PlayerInfoType));
	  {$I+}
	  if ioresult<>0 then
	  begin
	    XPrintfCenterStars(160,220,VisiblePageOffs,BaseColor,'Cannot open file '''+demofilename+'''');
	    demofilename:='xquest.dmo';
	    SetDemoFile(demofilename);
            WaitForEvent(Stars);
	  end
	  else
	  begin
	    recording:=true;
{$IFDEF DEBUG}
	  writeln(debugfile);
	  writeln(debugfile,'Recording file ',demofilename);
{$ENDIF}
	    Game;
	  end;
	end;
      end;
   end;
end;

procedure RunGame;
var i:integer;
    ch:char;

const Menu1:MenuListType=(num:6;s:((name:'Start Game';mtype:item),
				   (name:'Help';mtype:item),
				   (name:'Hall of Fame';mtype:item),
				   (name:'Options';mtype:item),
				   (name:'Demo';mtype:item),
				   (name:'Quit';mtype:item),
                                   (name:'';mtype:item)));



begin
  MenuScreenSetup;
  if not JoyAPresent then  PlayerInfo[Player].InputDevice:=MouseInput;
  if not MousePresent then  PlayerInfo[Player].InputDevice:=JoyInput;
  if JoyAPresent and (not JoyStickACalibrated) then
  if PlayerInfo[Player].InputDevice=JoyInput then
  begin
    InitialiseJoyStick(130);
    WriteDefaults;
    ClearInputBuffers(false);
  end;
  i:=0;
  repeat
    i:=XMenu(Menu1,i,30);
    case i of
      MenuTimeOut:
	begin
	  demofilename:='xquest.dmo';
	  if SetDemoFile(demofilename) then
	  begin
	    SavedPlayerInfo:=PlayerInfo;
	    PlayerInfo:=DemoPlayerInfo;
	    demomode:=true;
	    game;
	    PlayerInfo:=SavedPlayerInfo;
	    GameMode:=SavedGameMode;
	  end;
	  i:=0;
	end;
      0:Game;
      1:ShowHelp;
      2:HiScores(120);
      3:OptionsMenu;
      4:DemoMenu;
    end;
    if (i=5) or (i=NoItemSelected) then
    begin
      ResetStars(10,90,230,310);
      XRectFill(10,90,230,310,VisiblePageOffs,0);
      XPrintfCenterStars(160,120,VisiblePageOffs,BaseColor,'Are you sure you');
      XPrintfCenterStars(160,140,VisiblePageOffs,BaseColor,'want to quit?');
      ch:=YesNoStars('N');
    end;
  until ((i=5) or (i=NoItemSelected)) and (ch='Y');
end;

begin
{$IFDEF TESTCOLLIDEBITMAPS}
  TestCollideBitmaps;
{$ENDIF}
{$IFDEF DEBUGDEMO}
  assign(debugfile,'debug.txt');
  rewrite(debugfile);
{$ENDIF}
  Initialise;
  InitialiseGame;   {now in SetXModeNoSplitScreen mode}
  RunGame;
  WriteDefaults;
  NormalTermination:=true; {lets the exit procedure print the closing message,
			    which otherwise obscures any error messages}
{$IFDEF DEBUGDEMO}
  close(debugfile);
{$ENDIF}

end.
