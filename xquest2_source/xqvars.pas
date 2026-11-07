{$UNDEF BONUSLEVEL}

{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*            Copyright (C) 1994 M.Mackey. All rights reserved.              *}
{*                                                                           *}
{*                Variable, type and constant declarations                   *}
{*                                                                           *}
{*****************************************************************************}

unit xqvars;
interface
uses keyboard,joystick,mouse,filexist,sbunit;

type PlayerType=(Player1,Player2);
     GameModeType=(OnePlayer,TwoPlayer);
     InputDeviceType=(MouseInput,JoyInput,KeyBoardInput);
     KeysType=(UpKey,DownKey,LeftKey,RightKey,UpLeftKey,UpRightKey,
               DownLeftKey,DownRightKey,BrakeKey,FireKey,SmartBombKey);
     GateType=(Left,Right);
     PlayerInfoType=array[Player1..Player2] of
		   record
		      InputDevice:InputDeviceType;
		      HInputSpeed:integer;
                      VInputSpeed:integer;
                      JoyFireButton,JoySmartBombButton:integer;
                      MouseFireButton,MouseSmartBombButton:integer;
		      DiffLevel:integer;
		      KeyArray:array[UpKey..SmartBombKey] of integer;
		    end;

const PlayerInfo:PlayerInfoType=
	((InputDevice:MouseInput; HInputSpeed:64; VInputSpeed:64;
            JoyFireButton:JoyStickAButton1; JoySmartBombButton:JoyStickAButton2;
            MouseFireButton:MouseButton1; MouseSmartBombButton:MouseButton2;
            DiffLevel:2; KeyArray:
            (kKEYPAD8,kKEYPAD2,kKEYPAD4,kKEYPAD6,kKEYPAD7,kKEYPAD9,
             kKEYPAD1,kKEYPAD3,kKEYPAD5,kENTER,kSPACE)),
	 (InputDevice:MouseInput; HInputSpeed:64; VInputSpeed:64;
            JoyFireButton:JoyStickAButton1; JoySmartBombButton:JoyStickAButton2;
            MouseFireButton:MouseButton1; MouseSmartBombButton:MouseButton2;
            DiffLevel:2; KeyArray:
            (kKEYPAD8,kKEYPAD2,kKEYPAD4,kKEYPAD6,kKEYPAD7,kKEYPAD9,
             kKEYPAD1,kKEYPAD3,kKEYPAD5,kENTER,kSPACE)));

const
{*****}MemoryRecommended=130000;
      MemoryRequired=115000;
{*****}
      maxmissiles=50;           {absolute maximum # missiles}
      currmaxmissiles:byte=50;  {actual <variable> maximum #missiles}
				{  (vary with -slow or play level)   }
      MissileLife=300;
      MaxObjects=65;                    { check if large enough. Each takes ~300 bytes}
      Ticks=180;                        { # timer ticks/second}
      MaxEnemyMissiles=70;
      CurrMaxEnemyMissiles:byte=70;     {varies with -slow param}
      MaxEnemyMines=60;
      MaxEnemyFrames=5;                 {maximum # of enemy frames(starting at 0)}
      MaxEnemies=40;
      MaxEnemyKinds=18;
      maxlevel=50;
      MaxGameClocked=5;
      maxmissilekinds=6;
      MaxShipPics=24;
      MaxShipSpeed=640;                 {NOTE: Make difficulty level dependent?}
      BaseGameSpeed:integer=64;         {base game speed variable}
      Framerate=67;                     {approx rate of vsync}
      MaxDemoFrames=13100;
      MaxSounds=25;
      SoundsLoaded:boolean=false;
      SoundsOn:boolean=true;
      JoyStickACalibrated:boolean=False;
      JoyStickBCalibrated:boolean=False;
      ShowCountDown:boolean=True;
      GameMode:GameModeType=OnePlayer;
      StartShipDestroyedCount=64;
      MaxDiffLevel=5;
const DiffName:array[0..MaxDiffLevel-1] of string[7]=
	('Wimp   ','Timid  ','Average','Tricky ','Inhuman');
const DiffName2:array[0..MaxDiffLevel-1] of string[7]=
	('WIMP','TIMID','AVERAGE','TRICKY','INHUMAN');
const PlayerStr:array[Player1..Player2] of string[8]=
	('PLAYER 1','PLAYER 2');
const GameClockedNames:array[1..MaxGameClocked] of string[22]=
	('XQUEST WARRIOR','XQUEST WARRIOR SUPREME',
	 'XQUEST COMMANDER','XQUEST WARLORD','XQUEST GOD');

type ShortString=string[4];
type PowerUpType=(Shield,AimedFire,RapidFire,MultiFire,AssFire,HeavyFire,Bounce);
type PowerUpRecord=record
		     value,position:word;   {A position or value of 0
					     indicates not shown}
		     TimeMin,TimeRan:word;
		     pic:pointer;
		   end;
const PowerUp:array[Shield..Bounce] of PowerUpRecord=
	((value:0;Position:0;TimeMin:10*FrameRate;TimeRan:15*FrameRate;pic:nil),
	 (value:0;Position:0;TimeMin:30*FrameRate;TimeRan:60*FrameRate;pic:nil),
	 (value:0;Position:0;TimeMin:60*FrameRate;TimeRan:90*FrameRate;pic:nil),
	 (value:0;Position:0;TimeMin:60*FrameRate;TimeRan:90*FrameRate;pic:nil),
	 (value:0;Position:0;TimeMin:60*FrameRate;TimeRan:90*FrameRate;pic:nil),
	 (value:0;Position:0;TimeMin:60*FrameRate;TimeRan:90*FrameRate;pic:nil),
	 (value:0;Position:0;TimeMin:30*FrameRate;TimeRan:60*FrameRate;pic:nil));

      SuperTimeMin=5*FrameRate;
      SuperTimeRan=5*Framerate;   {min. and random duration of supercrystals}

const GameClockedSpeedUp:array[0..MaxGameClocked] of real=(0, 0.2 , 0.5 , 1 , 1.5 , 2);

const enemyname:array[1..MaxEnemyKinds] of string[12]=
	('Explosion','Grunger','Zippo','Zinger','Vince','','Miner',
	 'Meeby','Retaliator','Terrier','Doinger','Snipe','Tribbler',
	 'Tribble','Buckshot','Cluster','Sticktight','Repulsor');

const NormalTermination:boolean=False;

const StartBombs:integer=3;
      StartLives:integer=3;
      StartLevel:integer=1;

      MaxSpriteWidth=24;
      MaxSpriteHeight=24;
      MaxFontEntries=40;
      FontEntrySize=116;

      PageHeight=320;
      PageWidth=392;
      PhysicalPageWidth=320;
      SplitScreenLine=217;
      MaxVisiblePageY=PageHeight-SplitScreenLine;
      MaxVisiblePageX=PageWidth-PhysicalPageWidth;
      ScreenVBorder=SplitScreenLine div 2;
      ScreenHBorder=PhysicalPageWidth div 2-20;
	 {size of screen border for scrolling}

      MinGateX=20;
      MaxGateX=PageWidth-30;
      TimerX=280;                 {physical screen coords of countdown timer}
      TimerY=10;

      MineXMin=11;
      MineXMax=PageWidth-21;
      MineYMin=14;
      MineYMax=PageHeight-21;

      EnemyXMin=10;
      EnemyXMax=PageWidth-10;
      EnemyYMin=10;
      EnemyYMax=PageHeight-10;
      EnemyStartY=PageHeight div 2-5;

      ShipMaxX=PageWidth-11;
      ShipMaxY=PageHeight-11;
      ShipMinX=10;
      ShipMinY=10;
      ShipStartX=PageWidth div 2;
      ShipStartY=PageHeight div 2;





type point=record
	     x,y,xbr,ybr:integer;
	   end;
     objtype=(crys,mine,smart);
     objpoint=record
	     x,y,xbr,ybr:integer;
	     typ:objtype;
	     delete:boolean;
{$IFDEF BONUSLEVEL}
	     bonus:boolean;
{$ENDIF}
	     backgr:pointer;
	   end;
     masktype=array[0..MaxSpriteHeight-1] of longint;
     maskptr=^masktype;
     shiptype=record
		sx,sy:integer;
		delx,dely:integer;
		x,y,xbr,ybr,oldx,oldy:integer;
		    {sx,sy: coords shl 5
		     x,y:   screen coords of upper left corner
		     xbr,ybr:    "      "    lower right corner
		     delx,dely: velocity coords
		     oldx,oldy: old screen coords (ul corner)
		    }
		pic:array[0..MaxShipPics-1] of pointer;
		mask:array[0..MaxShipPics-1] of maskptr;
		ShipDir:integer;
		misspic:pointer;
		missmask:maskptr;
		backgr,oldbackgr:pointer;
		width,bmwidth,height,NumMissiles,MissWidth,MissBMWidth,MissHeight:integer;
	      end;
     ObjPosType=array[1..maxobjects] of objpoint;
     objecttype=record
		  pic:array[crys..smart] of pointer;
		  mask:array[crys..smart] of maskptr;
		  height,width,bmwidth:integer;
		  pos:ObjPosType;
		end;
const SizeOfMissileType=30;
type missiletype=record
		   x,y,xbr,ybr,sx,sy,delx,dely,oldx,oldy:integer;
		   time:integer;
		   backgr,oldbackgr:pointer;
		 end;
     emisskindtype=record
		 mspeed,soundnum,width,bmwidth,height:integer;
		 rebound,firedirect:boolean;
		 pic:pointer;
		 mask:maskptr;
	       end;
const SizeOfEmissileType=48;  {must be EVEN and kept up to date}
type emissiletype=record
		    x,y,xbr,ybr,sx,sy,delx,dely,oldx,oldy:integer;
		    mtyp:emisskindtype;
		    backgr,oldbackgr:pointer;
		  end;
     eminetype=record
		 pos:array[1..MaxEnemyMines] of point;
		 pic:pointer;
		 mask:maskptr;
		 adding:boolean;
		 height,width,bmwidth:integer;
	       end;
     enemykindtype=record
		 speed,speed2,curve,curve2,hits,firetype,score,deathsound:integer;
		 fires,follows,curves,explodes,laysmines,shootback,zoom,
		   maxspeed,rebounds,tribbles,repulses:boolean;
		 fireprob,changedir,changecurve,follow:real;
		 pic:array[0..MaxEnemyFrames] of pointer;
		 mask:array[0..maxEnemyFrames] of maskptr;
		    {better as linked list}
		 width,height,bmwidth,numframes,framespeed:integer;
	       end;

{NB Type zero is a supercrystal, type 1 is an explosion}

const SizeOfEnemyType=149;
type enemytype=record
		 x,y,xbr,ybr,sx,sy,delx,dely,oldx,oldy:integer;
		 curvecos,curvesin:integer;
		 typ:EnemyKindType;
		 ntyp,hit,frame:integer;
		 supertime:integer;  {zero if not a supercrystal or explosion}
		 backgr,oldbackgr:pointer;
	       end;
     leveltype=record
		 numcryst,nummine,maxsmart:byte;
		 smartprob:real;
		 newman:longint;
		 maxenemies:byte;
		 erelease:real;
		 GateWidth:integer;
		 GateMove:integer;
		 GateChangeDirProb:real;
		 Time:word;
		 BonusLevel:boolean;
	       end;
     DiffInfoType=record                {information on difficulty levels}
		    rebound:boolean;
		    speedfactor,enemyfrequency:real;
		  end;
     scoretype=record
		 score:longint;
		 level:word;
		 name:string[20];
	       end;
    Soundrec=record  {length 8 bytes}
		sample:pointer;
		length:word;
		position:longint;
	      end;
     GateRec=record
	       X,OldX,width,bmwidth,height:integer;
	       pic:pointer;
	       mask:maskptr;
	     end;
     demorectype=record
		   delx,dely:integer;
		   but:byte;
		 end;
     demotype=array[1..MaxDemoFrames] of demorectype;
     PaletteEntryType=record
			R,G,B:byte;
		      end;
     SmallFontType=record
		     height,width,bmwidth:integer;
		     pic:array['0'..'9'] of pointer;
		   end;
     FontType=array[0..127] of record
				 height,width:integer;
				 pic:pointer;
			       end;
     TimeType=record
		x,y,oldx,oldy:integer;
		backgr,oldbackgr:pointer;
	      end;


const probs:array[1..maxlevel,0..maxenemykinds] of byte=
			     {*}                             {*}
{1}   ((5,0  ,60 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (5,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (5,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (5,0  ,15 ,85 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
{5}    (7,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,15 ,15 ,70 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,15 ,15 ,15 ,55 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
{10}   (7,0  ,15 ,15 ,15 ,15 ,0  ,50 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,10 ,10 ,10 ,10 ,0  ,10 ,60 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,10 ,10 ,10 ,10 ,0  ,10 ,3  ,60 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
{15}   (7,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,10 ,10 ,10 ,10 ,0  ,10 ,3  ,3  ,60 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,3  ,3  ,60 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (7,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ),
{20}   (10,0 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,5  ,3  ,3  ,60 ,0  ,0  ,0  ,0  ,0  ,0  ),
       (10,0 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,5  ,3  ,3  ,60 ,0  ,0  ,0  ,0  ,0  ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,5  ,5  ,3  ,3  ,0  ,60 ,0  ,0  ,0  ),
{25}   (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,5  ,5  ,3  ,0  ,3  ,60 ,0  ,0  ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,5  ,5  ,5  ,0  ,3  ,3  ,60 ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,5  ,5  ,0  ,3  ,3  ,50 ,0  ),
{30}   (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,5  ,5  ,40 ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,30 ,0  ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,100),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,5  ,5  ,5  ,0  ,3  ,3  ,3  ,60 ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,3  ,3  ,3  ,40 ),
{35}   (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,3  ,20 ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
	      {start of combo levels here}
       (10,0  ,0  ,100,0  ,100,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,50 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,50 ,0  ),
{40}   (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,50 ,0  ,0  ,0  ,0  ,50 ,0  ,0  ,0  ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,50 ,0  ,50 ,0  ,0  ,0  ,0  ,0  ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
{45}   (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,50 ,0  ,50 ,0  ,0  ,0  ,0  ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,50 ,50 ,0  ,0  ),
       (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ),
       (10,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,0  ,50 ,50 ),
{50}   (10,0  ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ,10 ,10 ,10 ,0  ,10 ,10 ,10 ,10 ));
{Clock here}

{NOTE: Current implementation limits maximum gatemove to 90. If this is
       to be increased then the gate sprites must be extended.}


const  levels:array[1..maxlevel] of leveltype=
{1}    ((numcryst:15;nummine:0;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:20;erelease:0.005;GateWidth:27;GateMove:0;
	   GateChangeDirProb:0;Time:20;BonusLevel:false),
	(numcryst:16;nummine:3;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:4;erelease:0.005;GateWidth:27;GateMove:0;
	   GateChangeDirProb:0;Time:20;BonusLevel:false),
	(numcryst:17;nummine:4;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:5;erelease:0.005;GateWidth:26;GateMove:0;
	   GateChangeDirProb:0;Time:20;BonusLevel:false),
	(numcryst:18;nummine:5;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:6;erelease:0.005;GateWidth:26;GateMove:0;
	   GateChangeDirProb:0;Time:25;BonusLevel:false),
{5}     (numcryst:19;nummine:6;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:7;erelease:0.005;GateWidth:25;GateMove:0;
	   GateChangeDirProb:0;Time:25;BonusLevel:false),
	(numcryst:20;nummine:6;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:8;erelease:0.005;GateWidth:25;GateMove:0;
	   GateChangeDirProb:0;Time:30;BonusLevel:false),
	(numcryst:21;nummine:7;maxsmart:1;smartprob:0.2;newman:15000;
	   maxenemies:9;erelease:0.005;GateWidth:24;GateMove:0;
	   GateChangeDirProb:0;Time:30;BonusLevel:false),
	(numcryst:22;nummine:7;maxsmart:1;smartprob:0.2;newman:20000;
	   maxenemies:10;erelease:0.006;GateWidth:24;GateMove:0;
	   GateChangeDirProb:0;Time:35;BonusLevel:false),
	(numcryst:23;nummine:8;maxsmart:1;smartprob:0.2;newman:20000;
	   maxenemies:10;erelease:0.006;GateWidth:23;GateMove:0;
	   GateChangeDirProb:0;Time:35;BonusLevel:false),
{10}    (numcryst:24;nummine:8;maxsmart:1;smartprob:0.2;newman:20000;
	   maxenemies:10;erelease:0.006;GateWidth:23;GateMove:0;
	   GateChangeDirProb:0;Time:40;BonusLevel:false),
	(numcryst:24;nummine:9;maxsmart:2;smartprob:0.2;newman:20000;
	   maxenemies:10;erelease:0.006;GateWidth:22;GateMove:0;
	   GateChangeDirProb:0;Time:40;BonusLevel:false),
	(numcryst:25;nummine:9;maxsmart:2;smartprob:0.2;newman:20000;
	   maxenemies:10;erelease:0.007;GateWidth:22;GateMove:0;
	   GateChangeDirProb:0;Time:45;BonusLevel:false),
	(numcryst:25;nummine:10;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.007;GateWidth:21;GateMove:0;
	   GateChangeDirProb:0;Time:45;BonusLevel:false),
	(numcryst:26;nummine:10;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.008;GateWidth:21;GateMove:0;
	   GateChangeDirProb:0;Time:45;BonusLevel:false),
{15}    (numcryst:26;nummine:10;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.008;GateWidth:20;GateMove:0;
	   GateChangeDirProb:0;Time:50;BonusLevel:false),
	(numcryst:27;nummine:11;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.009;GateWidth:20;GateMove:0;
	   GateChangeDirProb:0;Time:50;BonusLevel:false),
	(numcryst:27;nummine:11;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.009;GateWidth:19;GateMove:0;
	   GateChangeDirProb:0;Time:50;BonusLevel:false),
	(numcryst:28;nummine:11;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.01;GateWidth:19;GateMove:0;
	   GateChangeDirProb:0;Time:55;BonusLevel:false),
	(numcryst:28;nummine:12;maxsmart:2;smartprob:0.2;newman:40000;
	   maxenemies:10;erelease:0.01;GateWidth:18;GateMove:0;
	   GateChangeDirProb:0;Time:55;BonusLevel:false),
{20}    (numcryst:29;nummine:12;maxsmart:2;smartprob:0.3;newman:40000;
	   maxenemies:10;erelease:0.01;GateWidth:18;GateMove:0;
	   GateChangeDirProb:0;Time:55;BonusLevel:false),
	(numcryst:29;nummine:12;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:10;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:60;BonusLevel:false),
	(numcryst:30;nummine:13;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:10;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:60;BonusLevel:false),
	(numcryst:30;nummine:13;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:10;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:60;BonusLevel:false),
	(numcryst:31;nummine:13;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:10;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:60;BonusLevel:false),
{25}    (numcryst:31;nummine:13;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:10;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:65;BonusLevel:false),
	(numcryst:32;nummine:14;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:10;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:65;BonusLevel:false),
	(numcryst:32;nummine:14;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:11;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:65;BonusLevel:false),
	(numcryst:33;nummine:14;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:11;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:65;BonusLevel:false),
	(numcryst:33;nummine:14;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:12;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:70;BonusLevel:false),
{30}    (numcryst:34;nummine:15;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:12;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:70;BonusLevel:false),
	(numcryst:34;nummine:15;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:13;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:70;BonusLevel:false),
	(numcryst:35;nummine:15;maxsmart:2;smartprob:0.3;newman:70000;
	   maxenemies:13;erelease:0.01;GateWidth:17;GateMove:0;
	   GateChangeDirProb:0;Time:70;BonusLevel:false),
	(numcryst:35;nummine:15;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:14;erelease:0.01;GateWidth:17;GateMove:10;
	   GateChangeDirProb:0;Time:75;BonusLevel:false),
	(numcryst:36;nummine:16;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:14;erelease:0.01;GateWidth:17;GateMove:20;
	   GateChangeDirProb:0;Time:75;BonusLevel:false),
{35}    (numcryst:36;nummine:16;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:15;erelease:0.01;GateWidth:17;GateMove:30;
	   GateChangeDirProb:0;Time:75;BonusLevel:false),
	(numcryst:37;nummine:16;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:15;erelease:0.01;GateWidth:17;GateMove:40;
	   GateChangeDirProb:0;Time:75;BonusLevel:false),
{combo levels start here}
	(numcryst:37;nummine:16;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:16;erelease:0.025;GateWidth:17;GateMove:50;
	   GateChangeDirProb:0;Time:80;BonusLevel:false),
	(numcryst:38;nummine:17;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:16;erelease:0.011;GateWidth:17;GateMove:60;
	   GateChangeDirProb:0;Time:80;BonusLevel:false),
	(numcryst:38;nummine:17;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:17;erelease:0.012;GateWidth:17;GateMove:70;
	   GateChangeDirProb:0;Time:80;BonusLevel:false),
{40}    (numcryst:39;nummine:17;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:17;erelease:0.012;GateWidth:17;GateMove:80;
	   GateChangeDirProb:0;Time:80;BonusLevel:false),
	(numcryst:39;nummine:17;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:18;erelease:0.013;GateWidth:17;GateMove:90;
	   GateChangeDirProb:0.002;Time:85;BonusLevel:false),
	(numcryst:40;nummine:18;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:18;erelease:0.013;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.004;Time:85;BonusLevel:false),
	(numcryst:40;nummine:18;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:19;erelease:0.014;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.006;Time:85;BonusLevel:false),
	(numcryst:40;nummine:18;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:19;erelease:0.014;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.008;Time:85;BonusLevel:false),
{45}    (numcryst:40;nummine:18;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:20;erelease:0.015;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.010;Time:90;BonusLevel:false),
	(numcryst:40;nummine:19;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:20;erelease:0.016;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.012;Time:90;BonusLevel:false),
	(numcryst:40;nummine:19;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:20;erelease:0.016;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.014;Time:90;BonusLevel:false),
	(numcryst:40;nummine:19;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:20;erelease:0.017;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.016;Time:90;BonusLevel:false),
	(numcryst:40;nummine:19;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:20;erelease:0.017;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.018;Time:90;BonusLevel:false),
{50}    (numcryst:40;nummine:20;maxsmart:2;smartprob:0.3;newman:100000;
	   maxenemies:20;erelease:0.018;GateWidth:17;GateMove:100;
	   GateChangeDirProb:0.02;Time:90;BonusLevel:false));


const fontmap:array[#1..#128] of byte=  {maps ASCII to font no.}
      (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
       0,39,0,0,0,0,0,0,0,0,0,0,37,0,0,0,0,1,2,3,4,5,6,7,8,9,10,38,0,
       0,0,0,40,0,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,
       28,29,30,31,32,33,34,35,36,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
       0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0);

const SmartBombPal:array[1..11] of PaletteEntryType=
      (( R:  0; G:  0; B:  0 ),
       ( R:  8; G:  0; B:  0 ),
       ( R: 14; G:  0; B:  0 ),
       ( R: 20; G:  4; B:  0 ),
       ( R: 26; G: 12; B:  0 ),
       ( R: 32; G: 20; B: 10 ),
       ( R: 38; G: 28; B: 20 ),
       ( R: 44; G: 36; B: 30 ),
       ( R: 50; G: 44; B: 40 ),
       ( R: 56; G: 52; B: 50 ),
       ( R: 63; G: 63; B: 63 ));

const bcolor:array[1..5] of byte=(10,15,25,15,10);
       {colors for the border 'pipes'}

const DiffInfo:array[0..MaxDiffLevel-1] of DiffInfoType=
((rebound:true; speedfactor:0.7; enemyfrequency:0.7),
 (rebound:true; speedfactor:1; enemyfrequency:1),
 (rebound:false; speedfactor:1; enemyfrequency:1),
 (rebound:false; speedfactor:1.5; enemyfrequency:1.2),
 (rebound:false; speedfactor:2; enemyfrequency:1.5));

const fire6=1;    {sounds}
      fire5=2;
      phew=3;
      fire4=4;
      fire=5;
      boing=6;
      squelch=7;
      woohoo=8;
      allright=9;
      ohyeah=10;
      getcrystal=11;
      explosn=12;
      explosn2=13;
      explosn3=14;
      retaliate=15;
      ow=16;
      countdown=17;
      gatesound=18;
      sxtsmash=19;
      bark=20;
      applause=21;
      enemyent=22;
      menuclick=23;
      doh=24;
      repulse=25;

      NoSoundCard=0;
      SoundBlaster=1;
      GUS=2;
      SoundCard:integer=NoSoundCard;

      NoStars=false;
      Stars=true;

type GameInfoType=record
		    Level,TotalLevel:integer;                   {4}
		{Both count the number of levels played, but
		 Level clocks at MaxLevel}
		    NumSmartBombs,Lives,GameClocked:integer;    {6}
		    Score,NewManScore,LastNewManScore:longint;  {12}
		    TimeOnLevel:longint;                        {4}
		    Timing:boolean;                             {1}
		    NumObjects,NumCrystals,NumMines,NumSmarts:word; {8}
		    GateMovePos:boolean;                        {1}
		    AttractorX,AttractorY:integer;              {4}
		  end;
Const SizeofGameInfoType=40;


var Player:PlayerType;
    GameInfo:array[Player1..Player2] of GameInfoType;
    DemoPlayerInfo,SavedPlayerInfo:PlayerInfoType;
    SavedObjects:ObjPosType;
    SavedGameMode:GameModeType;

    GameOver,Quit,LevelFinished,ShipDestroyed,GateMoving,NoMines,
      PollOnTimer,DemoFileLoaded:boolean;

    TextWindowX,TextWindowY:word;

    NumEnemies,NumEnemyMissiles,NumEnemyMines,NumExplosions:word;

    VisiblePageX,VisiblePageY:integer;

    EnemyEnteringLeft,EnemyEnteringRight,EnemyLeftType,EnemyRightType:integer;

    Temp,TimeCount,FrameCount,GateMoveCount,SavedRandSeed:longint;

    SavedIntVec8,ExitSave:pointer;

    GameSpeed,SmartBombed,ShipDestroyedCount:integer;

{$IFDEF BONUSLEVEL}
    BonusLevelCount:integer;
{$ENDIF}

    f:text;

    enemy:array[1..maxenemies] of enemytype;
    enemykind:array[0..maxenemykinds] of enemykindtype;
    ship:shiptype;
    objects:objecttype;
    missiles:array[1..MaxMissiles] of missiletype;
    missiletemp:missiletype;
    emissiles:array[1..MaxEnemyMissiles] of emissiletype;
    emissiletemp:emissiletype;
    emisskind:array[1..MaxMissileKinds] of emisskindtype;
    emines:eminetype;
    font:array[1..maxfontentries] of pointer;
    SmallFont:SmallFontType;
    ComixFont:FontType;
    TimeRecord:TimeType;

    ShipPic,SmartPic,CrystalPic,StartPicPBM,WinBackGround,Attractor:pointer;

{$IFDEF BONUSLEVEL}
    BCrystalPic:pointer;
{$ENDIF}

    cost,sint:array[1..15] of integer;

    demomode,recording:boolean;
    demo:^demotype;
    demoptr,demoparts:word;
    demofilename:string[40];
    demofile:file;

{$IFDEF DEBUG}
    debugfile:text;
{$ENDIF}

{$IFDEF TESTMEM}
    memfile:text;
{$ENDIF}

    digsounds:array[1..MaxSounds] of SoundRec;

    Gate:Array[Left..Right] of GateRec;

    TLCorner,TRCorner,BRCorner,BLCorner:pointer;
    LEnemyGate,REnemyGate:array[0..5] of pointer;

    {$IFDEF GUS}
    GUSPresent:boolean;
    {$ENDIF}

{$I palette.inc}
{$I blank.inc}
{$I titlemap.inc}

implementation

begin
{$IFDEF DEBUG}
  assign(debugfile,'xqdebug.txt');
  if not exist('xqdebug.txt') then rewrite(debugfile) else append(debugfile);
{$ENDIF}

{$IFDEF TESTMEM}
  assign(memfile,'xqmem.txt');
  if not exist('xqmem.txt') then rewrite(memfile) else
  begin
    append(memfile);
    writeln(memfile);
    writeln(memfile,'---Start of new game---');
  end;
{$ENDIF}

end.
