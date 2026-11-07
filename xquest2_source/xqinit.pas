{$DEFINE STARTPIC} {Put picture on menu screen}
{$UNDEF GUS}
{$UNDEF DEBUG}

{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*            Copyright (C) 1994 M.Mackey. All rights reserved.              *}
{*                                                                           *}
{*                          Initialisation Unit                              *}
{*                                                                           *}
{*****************************************************************************}

unit xqinit;
interface
uses crt,dos,xlib,xqvars,mouse,starunit,filexist,keyboard,
     sbunit,joystick {$IFDEF GUS} ,ultradrv {$ENDIF};

type itemtype=(item,slidebar);
     MenuString=record
		  name:string[30];
		  case mtype:itemtype of
		    slidebar: (min,max,value:integer)
	       end;

     MenuListType=record
		    num:integer;
		    s:array[0..6] of MenuString;
		  end;
const NoItemSelected=-1;
      MenuTimeOut=-2;
      BaseColor=186;
      HighLightColor=188;
var   TextWindowMemSize:word;   {size of memory allocated to store
				 text window background}
      TextWindowSize:word;      {size of window}

procedure MouseSetup;
procedure InputMovement(var delx,dely:integer;CheckInputType,AllowMouse,SlowJoy,AllowKeyboard:boolean);
function AdlibPresent:boolean;
{$IFDEF GUS}
function TestAndInitialiseGus:boolean;
{$ENDIF}
procedure strhex(i:word;var s:ShortString);
function  parseBLASTER(var addr,irq:word;var dma:byte):boolean;
procedure PlaySound(s:integer);
procedure SetExitProcedure;
procedure TextWindow(Text:string);
procedure RemoveTextWindow;
procedure InitialiseEnemies;
procedure InitialiseGraphics;
procedure TitlePage;
procedure InitialiseVariables;
procedure InitialiseSounds;
procedure InitialiseJoystick(Y:integer);
procedure InitialiseMouse(Y:integer);
procedure InitialiseKeyboard(Y:integer);
procedure XCorner1(X,Y,S,C1,C2:integer;Page:word);
procedure XCorner2(X,Y,S,C1,C2:integer;Page:word);
procedure XWindow(X1,Y1,X2,Y2,BS,Page:word);
procedure DrawBox(X1,Y1,X2,Y2:integer;Page:word);
procedure XText(X,Y:integer;Box:boolean;Page:word;s:string);
procedure XTextCenter(X,Y:integer;Box:boolean;Page:word;s:string);
procedure MyDelay(Msec:word);
function  DelayOrEvent(TimeInTicks:longint):boolean;
procedure SetXMode;
procedure SetXModeNoSplitScreen;
procedure XGetMissPBM(X,Y:word;SrcHeight:byte;ScrnOffs:word;var Bitmap);
Procedure XPutMissPbm( X,Y,ScrnOffs:word; var Bitmap );
procedure XPrintf(X,Y,Page,Color:word;s:string);
procedure XPrintfCenter(Center,Y,Page,Color:word;s:string);
procedure XPrintfCenterStars(Center,Y,Page,Color:word;s:string);
procedure ClearInputBuffers(Stars:boolean);
procedure WaitForEvent(Stars:boolean);
function YesNoStars(default:char):char;
Function ButtonPressed:integer;
function ButtonDown:integer;
function GetString(X,Y,MaxLength,MaxScreenLength,Page,Color:word):string;
function GetFileName(X,Y,Page,Color:word):string;
procedure MenuScreenSetup;
function XMenu(var Items:MenuListType;InitialSelection:integer;TimeOut:word):integer;
{procedure XDrawSlideBar(X,Y,Page,Color,Min,Max:integer; var value:integer);}
procedure ShowHelp;
procedure HiScores(seconds:word);
procedure EndScreen;

implementation
type proctype=procedure;
const period=65535 div (ticks div 18);
var proc:proctype;
    timsum:longint;

procedure MouseSetup;
begin
  if MousePresent then
  begin
    mhlimit(0,1000);
    mvlimit(0,1000);
    mput(500,500);
  end;
end;

procedure InputMovement(var delx,dely:integer;CheckInputType,AllowMouse,SlowJoy,AllowKeyboard:boolean);
var mx,my:integer;
begin
  if (not CheckInputType) or (PlayerInfo[Player].InputDevice=JoyInput) then
  with PlayerInfo[Player] do
  begin
    if JoyStickACalibrated then
    begin
      if not SlowJoy then
      begin
	JoyPos(1,mx,my);
	delx:=(mx*HInputSpeed) div 64;
	dely:=(my*VInputSpeed) div 64;
      end else
      begin
	JoyPos(1,mx,my);
	delx:=(longint(delx)*32+mx) div 32;
	dely:=(longint(dely)*32+my) div 32;
      end;
    end;
  end;
  if (not CheckInputType) or AllowMouse or (PlayerInfo[Player].InputDevice=MouseInput) then
  with PlayerInfo[Player] do
  begin
    mmotion(mx,my);
    mx:=(mx*HInputspeed) div 64;
    my:=(my*VInputspeed) div 64;
    delx:=delx+mx;
    dely:=dely+my;
  end;
  if CheckInputType and (PlayerInfo[Player].InputDevice=KeyboardInput) and AllowKeyBoard then
  with PlayerInfo[Player] do
  begin
    if keys[KeyArray[LeftKey]] or keys[KeyArray[DownLeftKey]]
       or keys[KeyArray[UpLeftKey]] then delx:=delx-(10*HInputSpeed div 64);
    if keys[KeyArray[RightKey]] or keys[KeyArray[DownRightKey]]
       or keys[KeyArray[UpRightKey]] then delx:=delx+(10*HInputSpeed div 64);
    if keys[KeyArray[UpKey]] or keys[KeyArray[UpLeftKey]]
       or keys[KeyArray[UpRightKey]] then dely:=dely-(10*VInputSpeed div 64);
    if keys[KeyArray[DownKey]] or keys[KeyArray[DownLeftKey]]
       or keys[KeyArray[DownRightKey]] then dely:=dely+(10*VInputSpeed div 64);
    if keys[KeyArray[BrakeKey]] then
    begin
      delx:=round(delx/1.2);
      dely:=round(dely/1.2);
    end;

  end;
end;


procedure strhex(i:word;var s:ShortString);
const hexdigit:array[0..15] of char=
  ('0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F');
begin
  s:='';
  repeat
    s:=hexdigit[i and 15]+s;
    i:=i shr 4;
  until i=0;
end;

procedure CheckExistence(filename:string);
begin
  if not exist(filename) then
  begin
    XTextMode;
    writeln('The file ''',filename,''' could not be found. Please ensure');
    writeln('that this file is in the current directory when executing');
    writeln('XQUEST.');
    repeat until keypressed;
    halt;
  end;
end;

function inthex(var s:string;var i:word):boolean;
  {returns true if successful}
const hexdigit:array[0..15] of char=
  ('0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F');
      hexdigitset:set of char=['0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F'];
var j,k:byte;
begin
  j:=1;
  while (not (upcase(s[j]) in hexdigitset)) and (j<=length(s)) do inc(j);
  if j>length(s) then
  begin
    inthex:=false;
    exit;
  end;
  s:=copy(s,j,length(s));
  j:=0;i:=0;
  repeat
    inc(j);
    k:=0;
    while (k<16) and (upcase(s[j])<>hexdigit[k]) do
    begin
      inc(k);
    end;
    if k<16 then i:=i shl 4 + k;
  until (k>=16) or (j>=length(s));
  s:=copy(s,j,length(s));
  inthex:=true;
end;

function intdec(s:string; var i:word):boolean;
var code:integer;
begin
  s:=copy(s,1,2);
  if s[2]=' ' then s:=copy(s,1,1);
  val(s,i,code);
  intdec:=(code=0);
end;


function parseBLASTER(var addr,irq:word;var dma:byte):boolean;
var s,s2:string;
    i,j:word;
begin
  s:=getenv('BLASTER');
  if s='' then begin parseBLASTER:=false;exit;end
    else parseBLASTER:=true;
  i:=pos('a',s);if i=0 then i:=pos('A',s);
  s2:=copy(s,i+1,length(s));
  inthex(s2,addr);
  i:=pos('i',s);if i=0 then i:=pos('I',s);
  s2:=copy(s,i+1,length(s));
  intdec(s2,irq);
  i:=pos('d',s);if i=0 then i:=pos('D',s);
  s2:=copy(s,i+1,length(s));
  intdec(s2,j);
  dma:=j;
end;

procedure out(address,data:byte);
var a,b:byte;
begin
  port[$0388]:=address;
  for a:=1 to 6 do b:=port[$0388];
  port[$0389]:=data;
  for a:=1 to 35 do b:=port[$0388];
end;

function AdLibPresent:boolean;
var status,status2:byte;
begin
  out(4,$60);
  out(4,$80);
  status:=port[$0388];
  out(2,$FF);
  out(4,$21);
  crt.delay(200);
  status2:=port[$0388];
  out(4,$60);
  out(4,$80);
  if ((status and $e0)=0) and ((status2 and $e0)=$c0) then AdlibPresent:=true
    else AdlibPresent:=false;
{  snum:=1;}
end;

{$IFDEF GUS}
function TestAndInitialiseGus:boolean;  {text mode proc}
begin
  TestAndInitialiseGUS:=true;

  {if not UltraProbe($220) then writeln('Not present at 220h');
  readln;}

  if not Ultra_Installed then
  begin
    TestAndInitialiseGus:=false;
    writeln(#13,#10,'ULTRASND environment variable not found.');
    writeln('GUS either not present or not used.');
  end
  else
  if not UltraProbe(Ultra_Config.Base_Port) then
  begin
    writeln(#13,#10,'Couldn''t find a GUS at the port specified in the ULTRASOUND');
    writeln('environment variable');
    delay(500);
    TestAndInitialiseGUS:=false;
  end
  else
  if not UltraOpen(Ultra_Config,MaxSounds) then
  begin
    writeln(#13,#10,'Couldn''t initialize the GUS.');
    delay(500);
    TestAndInitialiseGUS:=false;
  end
  else
  if not UltraReset(MaxSounds) then
  begin
    Writeln('Couldn''t reset the GUS.');
    TestAndInitialiseGUS:=false;
  end;
end;
{$ENDIF}


procedure PlaySound(s:integer);
begin
{$IFDEF GUS}
  if (not SoundsOn) or (not (SoundBlasterInitialised or GUSPresent)) or (s=0) then exit;
{$ELSE}
  if (not SoundsOn) or (not SoundBlasterInitialised) or (s=0) then exit;
{$ENDIF}
{$IFDEF DEBUG}
  if (s<0) or (s>MaxSounds) then
  begin
    XTextMode;
    writeln('Incorrect sound number ',s);
    readln;
    halt;
  end;
{$ENDIF}
  if SoundBlasterInitialised then
    AddSound(digsounds[s].sample^,digsounds[s].length);
{$IFDEF GUS}
  if GUSPresent then
  with digsounds[s] do
    UltraStartVoice(s,position,position,position+length,0);
{$ENDIF}
end;

(*
const maxfmsounds=16;
const offset:array[0..8] of byte=(0,1,2,8,9,10,16,17,18);
const sounds:array[1..maxfmsounds,1..13] of byte=
((5,0,118,152,0,1,0,246,136,0,152,2,33),               { scatterfire}
   (1,0,118,152,0,15,0,102,136,0,152,2,53),             { slofire}
   (1,0,52,116,0,13,0,37,136,0,152,0,33),               {leavelevel}
   (1,0,134,193,0,2,0,94,118,0,152,4,49),               {directfirer}
   (1,5,152,238,0,2,0,231,227,0,152,6,49),              {shooting}
   (5,21,132,182,2,8,64,166,87,2,202,0,41),             {doings}
   (2,0,111,15,1,0,0,246,40,3,174,0,34),                {oink}
   (2,0,31,212,0,0,0,163,156,2,152,6,49),               {newman}
   (1,0,160,119,0,1,0,164,199,0,174,0,50),              {get smart}
   (192,0,163,183,1,210,0,164,193,0,32,7,54),           {get supercrystal}
   (15,135,91,226,1,10,3,202,62,3,107,0,45),            {get crystal}
   (11,66,128,138,0,9,10,35,4,1,129,14,57),             {lasing}
   (9,132,97,143,2,9,0,102,132,2,129,14,37),            {explosion}
   (2,7,150,248,2,10,0,169,39,1,152,0,49),              {shootback}
   (3,0,216,107,0,3,2,210,107,0,152,0,33),              {hit enemy}
   (193,19,118,75,3,193,19,118,171,3,152,1,49));        {countdown of timebonus}


  if soundnum=0 then
  begin
    snum:=(snum+1) mod 8;
    soundnum:=snum;
  end;
  offs:=offset[soundnum];
  out($b0+soundnum,sounds[s,13] and $DF);
  out($20+offs,sounds[s,1]);
  out($40+offs,sounds[s,2]);
  out($60+offs,sounds[s,3]);
  out($80+offs,sounds[s,4]);
  out($e0+offs,sounds[s,5]);
  out($23+offs,sounds[s,6]);
  out($43+offs,sounds[s,7]);
  out($63+offs,sounds[s,8]);
  out($83+offs,sounds[s,9]);
  out($e3+offs,sounds[s,10]);
  out($a0+soundnum,sounds[s,11]);
  out($c0+soundnum,sounds[s,12]);
  out($b0+soundnum,sounds[s,13]);
end;
*)

procedure writeln2(c,b:byte;s:string);
var i:integer;
begin
  textbackground(0);
  write('     ');
  textbackground(b);
  textcolor(c);
  for i:=1 to ((70-length(s)) div 2) do write(' ');
  write(s);
  for i:=((70-length(s)) div 2)+length(s) to 70 do write(' ');
  writeln;
end;




{$F+}
procedure myexit;  {restores changed interrupt vectors & returns to textmode}
var i,j:integer;
    s:string;
begin
  asm
    mov al,36h  { program timer 0 with modus 3 }
    out 43h,al  { and counter value of 0 (2^16)}
    mov al,0
    out 40h,al
    out 40h,al
  end;
  setintvec(8,savedintvec8);
  mstatus(i,j);   {reset mouse}
  if CurrXMode<>XModeText then XTextMode;

  TextMode(LastMode);


{$IFDEF GUS}
  if GUSPresent then
    if not UltraClose then
      writeln('Unable to properly shut down GUS card');
{$ENDIF}

{$IFDEF TESTMEM}
  close(memfile);
{$ENDIF}

{$IFDEF DEBUG}
   close(debugfile);
{$ENDIF}

  if SoundBlasterInitialised then
  begin
    SbDone;
  end;

  writeln;

  if NormalTermination then
  begin
    writeln2(15,1,'XQUEST 2');
    writeln2(11,1,'Copyright (C) 1996 Mark Mackey (Atomjack).');
    writeln2(14,1,'컴컴컴컴컴컴컴컴컴컴컴컴컴컴컴컴컴�');
    writeln2(14,1,'');
    writeln2(14,1,'If you enjoyed this game then a donation of UKP10 (US$15) would');
    writeln2(14,1,'be much appreciated and would encourage me to continue supporting');
    writeln2(15,1,'XQUEST.');
    writeln2(14,1,'');
    writeln2(14,1,'You can contact me by mail as');
    writeln2(14,1,'');
    writeln2(14,1,'Mark Mackey');
    writeln2(14,1,'c/o Trinity Hall,');
    writeln2(14,1,'Cambridge CB2 1TJ,');
    writeln2(14,1,'UK');
    writeln2(14,1,'');
    writeln2(14,1,'or by email as mdm1004@cus.cam.ac.uk');
    writeln2(14,1,'');
    writeln2(14,1,'Please send any comments, suggestions, praise, flames, and bug');
    writeln2(14,1,'reports to me. Any and all feedback is welcome.');
    writeln2(14,1,'');
    writeln2(14,1,'I hope you had as much fun playing this game as I had making it!');
    writeln2(14,1,'');
  end;
  exitproc:=exitsave;
end;
{$F-}

{$F+}
procedure Timer;
interrupt;
begin
  asm
    cli      {clear interrupts}
    cmp word ptr [timsum+2],0
    jl  @DontCallOld
    dec word ptr [timsum+2]
    mov al,byte ptr [SoundBlasterInitialised]
    and al,byte ptr [PollOnTimer]
    jz  @DontPollSB
    call sbpoll             {poll SB on original timer tick 18.2 Hz}
@DontPollSB:
    pushf
    call proc                   {original timer interrupt}
@DontCallOld:
    add  word ptr [timsum],period
    adc  word ptr [timsum+2],0
    add  word ptr [TimeCount],1
    adc  word ptr [TimeCount+2],0
    mov al,20h;  {signal end of interrupt (EOI) to interrupt controller}
    out 20h,al;
    sti;
  end;
end;
{$F-}

{$F+}
function HeapFunc(Size:word):Integer; far;
begin
  if Size<>0 then
  begin
    if CurrXMode<>XModeText then XTextMode;
    writeln('Out of Memory!');
    writeln;
    writeln('Memory allocation request of ',Size,' bytes could not be met.');
    writeln('XQuest requires more memory to run properly. Sorry!');
    writeln('Try removing some TSRs (disk caches, networking software, DOS etc.),');
    writeln(' or run XQuest without sound.');
    writeln;
    halt(2);
  end;
  HeapFunc:=0;
end;
{$F-}

procedure SetExitProcedure;
  {Sets up exit procedure and reprograms timer}
begin
  exitsave:=exitproc;
  exitproc:=@myexit;
  timecount:=0;
  getintvec(8,savedintvec8);
  timsum:=65536;
  asm
    mov al,36h  { program timer 0 with modus 3 }
    out 43h,al  { and counter value of period  }
    mov ax,period
    out 40h,al
    mov al,ah
    out 40h,al
  end;
  proc:=proctype(savedintvec8);   {proc points to old interrupt vector}
  setintvec(8,@Timer);
  HeapError:=@HeapFunc;           {install new Heap error handler}
end;

procedure TextWindow(Text:string);
var l:integer;
begin
  l:=length(text)*4;
  TextWindowSize:=l;
  TextWindowMemSize:=(2*l+24)*30+2;
  getmem(WinBackGround,TextWindowMemSize);
{$IFDEF TESTMEM}
      writeln(memfile,'Window allocation: ',TextWindowMemSize,' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
  XGetPBM(160+VisiblePageX-l-10,95+VisiblePageY,(2*l+20) div 4 +1,30,VisiblePageOffs,WinBackGround^);
  XWindow(160+VisiblePageX-l-10,95+VisiblePageY,
	  160+VisiblePageX+l+10,125+VisiblePageY,2,VisiblePageOffs);
  XText(TextWindowX+10,TextWindowY+8,False,VisiblePageOffs,Text);
end;

procedure RemoveTextWindow;
begin
  XPutPBM(160+VisiblePageX-TextWindowSize-10,95+VisiblePageY,
	  VisiblePageOffs,WinBackGround^);
  freemem(WinBackGround,TextWindowMemSize);
end;


Procedure XGetMissPbm( X,Y: word;SrcHeight:byte;
		 ScrnOffs:word; var Bitmap ); assembler;
var  LineInc:word;
asm
	push  ds
	cld
	mov   bx,Y
	shl   bx,1
	mov   ax,word ptr [ScrOffsetTable+bx]
	mov   bx,ScrnLogicalByteWidth
	mov   si,ScrnOffs
	add   si,ax
	mov   cx,X
	mov   dx,cx
	shr   dx,2
	add   si,dx
	mov   ax,SCREENSEG
	mov   ds,ax
	les   di,Bitmap
	mov   al,1
	mov   ah,SrcHeight
	stosw
	xor   ah,ah
	sub   bx,ax
	mov   LineInc,bx
	and   cx,0003h
	mov   ah,11h
	shl   ah,cl
	mov   dx,GCINDEX
	mov   al,READMAP
	out   dx,al
	inc   dx
	mov   al,cl
	mov   bh,SrcHeight

@@PlaneLoop:  {unrolled}
     {use bh to save SrcHeight}
	mov   bl,bh
	out   dx,al
	mov   cx,si
@@RowLoop:
	movsb
	add   si,LineInc
	dec   bl
	jnz   @@RowLoop
	inc   al
	and   al,3
	rol   ah,1
	adc   cx,0

	mov   si,cx
	mov   bl,bh
	out   dx,al
@@RowLoop2:
	movsb
	add   si,LineInc
	dec   bl
	jnz   @@RowLoop2
	inc   al
	and   al,3
	rol   ah,1
	adc   cx,0

	mov   si,cx
	mov   bl,bh
	out   dx,al
@@RowLoop3:
	movsb
	add   si,LineInc
	dec   bl
	jnz   @@RowLoop3
	inc   al
	and   al,3
	rol   ah,1
	adc   cx,0

	mov   si,cx
	mov   bl,bh
	out   dx,al
@@RowLoop4:
	movsb
	add   si,LineInc
	dec   bl
	jnz   @@RowLoop4

	pop   ds
end;


Procedure XPutMissPbm( X,Y,ScrnOffs:word; var Bitmap ); assembler;
{Version of XPutPBM optimised for and specific to bitmaps of 4 pixel
 width or less}
var
	LineInc:word;
asm
	push  ds
	cld
	mov   ax,SCREENSEG
	mov   es,ax
	mov   bx,Y
	shl   bx,1
	mov   ax,word ptr [ScrOffsetTable+bx]
	mov   bx,ScrnLogicalByteWidth
	mov   di,ScrnOffs
	add   di,ax
	mov   cx,X
	mov   dx,cx
	shr   dx,2
	add   di,dx

	lds   si,Bitmap
	lodsw
	mov   dh,ah
	xor   ah,ah
	sub   bx,ax
	mov   LineInc,bx
	mov   bh,al

	and   cx,0003h

	mov   ah,11h
	shl   ah,cl
	mov   bh,dh
	mov   dx,SCINDEX
	mov   al,MAPMASK
	out   dx,al
	inc   dx

@@PlaneLoop:  {unrolled:}

	mov   cx,di
	mov   bl,bh
	mov   al,ah
	out   dx,al
@@RowLoop:
	movsb
	add   di,LineInc
	dec   bl
	jnz   @@RowLoop
	rol   ah,1
	adc   cx,0
	mov   di,cx

	mov   bl,bh
	mov   al,ah
	out   dx,al
@@RowLoop2:
	movsb
	add   di,LineInc
	dec   bl
	jnz   @@RowLoop2
	rol   ah,1
	adc   cx,0
	mov   di,cx

	mov   bl,bh
	mov   al,ah
	out   dx,al
@@RowLoop3:
	movsb
	add   di,LineInc
	dec   bl
	jnz   @@RowLoop3
	rol   ah,1
	adc   cx,0
	mov   di,cx

	mov   bl,bh
	mov   al,ah
	out   dx,al
@@RowLoop4:
	movsb
	add   di,LineInc
	dec   bl
	jnz   @@RowLoop4

	pop   ds
end;

procedure InitialiseEnemies;
var i:integer;
    f:file of enemykindtype;
begin
  CheckExistence('xquest.enm');
  assign(f,'xquest.enm');
  reset(f);
  for i:=0 to maxenemykinds do
  begin
    read(f,enemykind[i]);
  end;
  close(f);
end;

procedure InitialiseGraphics;
type longarray=array[1..60000] of byte;
var f:file;
    i,j,k,temp,maxtemp:integer;
    p:^longarray;
    ob:objtype;
    PU:PowerUpType;
    GT:GateType;

  procedure makemask(var p:longarray;width,bmwidth,height:integer;var mask:maskptr);
  var i,j,k,l:integer;
  const bits:array[0..31] of longint=
    ($8000,$4000,$2000,$1000,$800,$400,$200,$100,$80,$40,$20,$10,$8,$4,$2,$1,
     $80000000,$40000000,$20000000,$10000000,$8000000,$4000000,$2000000,
     $1000000,$800000,$400000,$200000,$100000,$80000,$40000,$20000,$10000);
  begin
    getmem(mask,height*4);   {get memory for mask}
    for i:=0 to (height-1) do
    begin
      mask^[i]:=0;
      for l:=0 to (bmwidth*4-1) do
	if p[i*bmwidth*4+l+3]<>0 then
	mask^[i]:=mask^[i] or (bits[l]);
    end;
(*
    begin
      XTextMode;
      for i:=0 to (height-1) do
      begin
	for j:=0 to 31 do if (mask^[i] and (bits[j]))=0 then
	  write('0') else write('1');
	write(' ');
	for j:=0 to bmwidth*4-1 do if (p[i*bmwidth*4+j+3]=0)  xor ((mask^[i] and (bits[j]))=0)  then
	  write('0') else write('1');
	writeln('  ',mask^[i]);
      end;
      writeln(height,' ',bmwidth*4,' ',width);
      readln;
      for i:=0 to (height-1) do
      begin
	for j:=0 to bmwidth*4-1 do
	  write(p[i*bmwidth*4+j+3]:2,' ');
	writeln;
      end;
      readln;
    end;
*)
  end;

  function readandcompilebitmap(var f:file;var p:longarray;
       var width,bmwidth,height:integer;var pic:pointer):word;
      {returns size of compiled bitmap}
  var i,j:integer;
      size:word;
  begin
{$IFDEF TESTMEM}
    writeln(memfile,'Avail: ',memavail,' Max. block:  ',maxavail);
    flush(memfile);
{$ENDIF}
    blockread(f,width,2);
    blockread(f,height,2);
    bmwidth:=((width-1) div 4)+1; {byte width}
    p[1]:=bmwidth*4;
    p[2]:=height;
    if p[1]*height>sizeof(longarray) then halt;
    blockread(f,p[3],p[1]*height);
      {file actually contains a width which is always a multiple of 4}
    size:=xsizeofcbitmap(ScrnLogicalByteWidth,p);
    getmem(pic,size);
    xcompilebitmap(ScrnLogicalByteWidth,p,pic^);
    readandcompilebitmap:=size;
  end;

  procedure readbitmapPBM(var f:file;var p:longarray; var width,bmwidth,height:integer;var pic:pointer);
  begin
    blockread(f,width,2);
    blockread(f,height,2);
    bmwidth:=((width-1) div 4)+1; {byte width}
    p[1]:=bmwidth*4;
    p[2]:=height;
    if bmwidth*4*height>sizeof(longarray) then halt;
    blockread(f,p[3],bmwidth*4*height);
      {file actually contains a width which is always a multiple of 4}
    getmem(pic,height*bmwidth*4+2);
    xbmtopbm(p,pic^);
  end;

  procedure readfont(var f:file;var font:fonttype);
  var b:byte;
      i,j,pos,bmwidth:integer;
      p:array[1..200] of byte;    {scratch pad}
  begin
    for i:=0 to 127 do
      with font[i] do
      begin
	width:=0;
	height:=0;
	pic:=nil;
      end;
    repeat
      blockread(f,b,1);
      with font[b] do
      begin
	for i:=1 to 200 do p[i]:=0;   {clear scratchpad}
	blockread(f,width,2);
	blockread(f,height,2);
	bmwidth:=(((width-1) div 4)+1)*4;
	getmem(pic,4+height*bmwidth);
	p[1]:=bmwidth;
	p[2]:=height;
	pos:=3;
	for j:=0 to height-1 do
	begin
	  blockread(f,p[pos],width);    {do one row, zero-padded to bmwidth}
	  pos:=pos+bmwidth;
	end;
	xbmtopbm(p,pic^);
      end;
    until eof(f);
  end;


begin
{$IFDEF TESTMEM}
  writeln(memfile,'Starting memory: ',maxavail);
{$ENDIF}

  getmem(p,6000);
  CheckExistence('xquest.gfx');
  assign(f,'xquest.gfx');
  reset(f,1);
{$IFDEF TESTMEM}
  writeln(memfile,'Starting Ship and Missiles. mem: ',memavail,' max: ',maxavail);
{$ENDIF}
  with ship do
  begin
    for i:=1 to MaxShipPics do
    begin
      readandcompilebitmap(f,p^,width,bmwidth,height,pic[i-1]);
      makemask(p^,width,bmwidth,height,mask[i-1]);
    end;
    getmem(backgr,bmwidth*height*4+4);
    getmem(oldbackgr,bmwidth*height*4+4);
    readandcompilebitmap(f,p^,misswidth,missbmwidth,missheight,misspic);
    makemask(p^,misswidth,missbmwidth,missheight,missmask);
{$IFDEF TESTMEM}
  writeln(memfile,'End of ship and missiles masks: ',maxavail);
{$ENDIF}
    for i:=1 to MaxMissiles do
    with missiles[i] do
    begin
      getmem(backgr,missbmwidth*missheight*4+4);
      getmem(oldbackgr,missbmwidth*missheight*4+4);
    end;
  end;
{$IFDEF TESTMEM}
  writeln(memfile,'End of ship and missiles ',maxavail);
{$ENDIF}
  maxtemp:=0;
  with objects do
  for ob:=crys to smart do
  begin
    readandcompilebitmap(f,p^,width,bmwidth,height,pic[ob]);
    makemask(p^,width,bmwidth,height,mask[ob]);
    temp:=bmwidth*height*4+4;
    if temp>maxtemp then maxtemp:=temp;
  end;
{$IFDEF TESTMEM}
  writeln(memfile,'End of object pictures: ',maxavail);
{$ENDIF}

  for j:=1 to MaxObjects do
  begin
    getmem(objects.pos[j].backgr,maxtemp);  {object backgrounds}
    getmem(SavedObjects[j].backgr,maxtemp);
  end;
	    {check enough memory is allocated...}
{$IFDEF TESTMEM}
  writeln(memfile,'End of objects ',maxavail);
{$ENDIF}

{$IFDEF BONUSLEVEL}
  readandcompilebitmap(f,p^,i,j,temp,bcrystalpic);
{$ENDIF}

  with emines do
  begin
    readandcompilebitmap(f,p^,width,bmwidth,height,pic);
    makemask(p^,width,bmwidth,height,mask);
  end;

  for i:=0 to maxenemykinds do
  with enemykind[i] do
    for j:=0 to numframes do
    begin
      readandcompilebitmap(f,p^,width,bmwidth,height,pic[j]);
      makemask(p^,width,bmwidth,height,mask[j]);
    end;

{$IFDEF TESTMEM}
  writeln(memfile,'End of enemy pictures ',maxavail);
{$ENDIF}

{**** Now Allocated dynamically
  for i:=1 to maxenemies do
  begin
    temp:=maxspritewidth*maxspriteheight+4;
    getmem(enemy[i].backgr,temp);
    getmem(enemy[i].oldbackgr,temp);
  end;
***}


{$IFDEF TESTMEM}
  writeln(memfile,'End of enemy backgrounds: ',maxavail);
{$ENDIF}

  maxtemp:=0;
  for i:=1 to MaxMissileKinds do
  with emisskind[i] do
  begin
    readandcompilebitmap(f,p^,width,bmwidth,height,pic);
    makemask(p^,width,bmwidth,height,mask);
    temp:=bmwidth*height*4+4;
    if temp>maxtemp then maxtemp:=temp;    {get maximum missile PBM size}
  end;

{$IFDEF TESTMEM}
  writeln(memfile,'End of emissile pictures: ',maxavail);
{$ENDIF}

  for i:=1 to MaxEnemyMissiles do
  with emissiles[i] do
  begin
    getmem(backgr,maxtemp);
    getmem(oldbackgr,maxtemp);
  end;

{$IFDEF TESTMEM}
  writeln(memfile,'End of emissile backgrounds: ',maxavail);
{$ENDIF}

  readbitmapPBM(f,p^,i,j,temp,ShipPic);
  readbitmapPBM(f,p^,i,j,temp,SmartPic);
  readbitmapPBM(f,p^,i,j,temp,CrystalPic);
  for PU:=Shield to Bounce do
    readbitmapPBM(f,p^,i,j,temp,PowerUp[PU].Pic);

  for GT:=Left to Right do
  with Gate[GT] do
  begin
    readbitmapPBM(f,p^,width,bmwidth,height,pic);
    makemask(p^,width,bmwidth,height,mask);
  end;
  readbitmapPBM(f,p^,i,j,temp,TLCorner);
  readbitmapPBM(f,p^,i,j,temp,TRCorner);
  readbitmapPBM(f,p^,i,j,temp,BRCorner);
  readbitmapPBM(f,p^,i,j,temp,BLCorner);
  for k:=0 to 5 do readbitmapPBM(f,p^,i,j,temp,LEnemyGate[k]);
  for k:=0 to 5 do readbitmapPBM(f,p^,i,j,temp,REnemyGate[k]);
  readbitmapPBM(f,p^,i,j,temp,Attractor);

{$IFDEF TESTMEM}
  writeln(memfile,'End of PBMs:  ',maxavail);
{$ENDIF}

  for k:=0 to 9 do
    with smallfont do
      readandcompilebitmap(f,p^,width,bmwidth,height,pic[char(48+k)]);
  with smallfont do
  begin
    getmem(TimeRecord.backgr,bmwidth*height*4*9+4);
    getmem(TimeRecord.oldbackgr,bmwidth*height*4*9+4);  {covers up to 5 characters}
  end;

{$IFDEF TESTMEM}
  writeln(memfile,'End of smallfont and TimeRecord: ',maxavail);
{$ENDIF}

  close(f);
  CheckExistence('xquest.fnt');
  assign(f,'xquest.fnt');
  reset(f,1);
  for i:=1 to maxfontentries do
  begin
    blockread(f,p^,fontentrysize);
    p^[2]:=p^[3]; {allow for word -> byte conversion}
    for j:=5 to 116 do p^[j-2]:=p^[j];
    getmem(font[i],116);
    xbmtopbm(p^,font[i]^);
  end;
  close(f);
  freemem(p,6000);

  CheckExistence('xquest2.fnt');
  assign(f,'xquest2.fnt');
  reset(f,1);
  readfont(f,comixfont);
  close(f);

{$IFDEF TESTMEM}
  writeln(memfile,'End of InitialiseGraphics: ',memavail);
{$ENDIF}
end;


procedure InitialiseVariables;
var i:integer;
    pl:playertype;
    pu:PowerUpType;
    f:file;
begin
  randomize;
  Player:=Player1;
  for pl:=Player1 to Player2  do
  with GameInfo[pl] do
  begin
    TimeOnLevel:=0;
    Timing:=false;
    Level:=StartLevel;
    TotalLevel:=Level;
    Score:=0;
    LastNewManScore:=Score;
    NumSmartBombs:=StartBombs;
    Lives:=StartLives;
    GameClocked:=0;             {No. of times the game has been 'clocked'}
  end;
  GameOver:=False;
  Quit:=False;
  PollOnTimer:=true;
  CheckBreak:=false;
  SmartBombed:=0;
  for PU:=Shield to Bounce do
  with PowerUp[PU] do
  begin
    value:=0;position:=0;
  end;
  ShipDestroyedCount:=StartShipDestroyedCount;
  EnemyEnteringLeft:=0;
  EnemyEnteringRight:=0;
  for i:=1 to 15 do
  begin
    cost[i]:=round(cos(i*168*pi/180)*32767);
    sint[i]:=round(sin(i*168*pi/180)*32767);
  end;
  with emisskind[1] do  {put in file}
  begin
    mspeed:=120;
    soundnum:=fire6;
    rebound:=false;
    firedirect:=false;
  end;
  with emisskind[2] do  {put in file}
  begin
    mspeed:=150;
    soundnum:=fire5;
    rebound:=false;
    firedirect:=false;
  end;
  with emisskind[3] do  {put in file}
  begin
    mspeed:=200;
    soundnum:=retaliate;
    rebound:=false;
    firedirect:=true;
  end;
  with emisskind[4] do  {put in file}
  begin
    mspeed:=150;
    soundnum:=0;
    rebound:=true;
    firedirect:=false;
  end;
  with emisskind[5] do  {put in file}
  begin
    mspeed:=150;
    soundnum:=fire4;
    rebound:=false;
    firedirect:=true;
  end;
  with emisskind[6] do  {put in file}
  begin
    mspeed:=170;
    soundnum:=0;
    rebound:=false;
    firedirect:=false;
  end;
end;

procedure InitialiseSounds;
var f:file;
    i:integer;
begin
  CheckExistence('xquest.snd');
  assign(f,'xquest.snd');
  reset(f,1);
  for i:=1 to MaxSounds do
  with digsounds[i] do
  begin
    blockread(f,length,2);
    getmem(sample,length);
{$IFDEF TESTMEM}
      writeln(memfile,'Sound allocation: ',length,' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
    blockread(f,sample^,length);
{$IFDEF GUS}
    if GUSPresent then
    begin
      if UltraMemAlloc(length,position) then
	if not UltraDownLoad(sample,Convert_Data,position,length,True) then
	begin
	  UltraMemFree(length,position);
	  if CurrXMode<>XModeText then XTextMode;
	  writeln('Error downloading sound file to GUS.');
	  halt(9);
	end;
      UltraSetBalance(i, 7);
      UltraSetLinearVolume(i,SoundVolume shl 3);  {range 0..511}
      UltraSetFrequency(i,kHz_11);
      freemem(sample, length);   {remove sample from main memory}
    end;
{$ENDIF}
  end;
  close(f);
  SoundsLoaded:=true;
end;



procedure InitialiseJoyStick(Y:integer);
var i:byte;
   {Runs Stars while operating!}

  procedure PositionOnButtonPress{(min:boolean)};
  var rtax,rtay:real;
  begin
    ReadJoyStick;
    rtax:=tax;rtay:=tay;
    ClearInputBuffers(Stars);
    repeat
      ReadJoyStick;
      rtax:=(rtax*8+tax)/9;
      rtay:=(rtay*8+tay)/9;
      StarFieldStep;
    until (JoyStickButtonPressed<>0) or (numkeypresses(kESC)>0);
    tax:=round(rtax);
    tay:=round(rtay);
  end;

begin
  if not JoyAPresent then
  begin
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Joystick not found');
    PlayerInfo[player1].InputDevice:=MouseInput;
    PlayerInfo[player2].InputDevice:=MouseInput;
    WaitForEvent(Stars);
    exit;
  end;

  with JoyCal[1] do
  begin
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Centre stick and press button');
    PositionOnButtonPress;
    if LastKeyHit=#27 then exit;
    XCentreMin:=round(tax*0.96);
    YCentreMin:=round(tay*0.96);
    XCentreMax:=round(tax*1.04);
    YCentreMax:=round(tay*1.04);
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Move to top left and press button');
    PositionOnButtonPress;
    if LastKeyHit=#27 then exit;
    XMin:=tax;YMin:=tay;
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Move to bottom right and press button');
    PositionOnButtonPress;
    if LastKeyHit=#27 then exit;
    XMax:=tax;YMax:=tay;
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'            Press fire button            ');
    repeat
      StarFieldStep;
      i:=JoyStickButtonPressed;
    until (i<>0) or (numkeypresses(kESC)>0);
    if LastKeyHit=#27 then exit;
    PlayerInfo[Player].JoyFireButton:=i;
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'          Press smartbomb button          ');
    repeat
      StarFieldStep;
      i:=JoyStickButtonPressed;
    until (i<>0) or (numkeypresses(kESC)>0);
    if LastKeyHit=#27 then exit;
    PlayerInfo[Player].JoySmartBombButton:=i;
    JoyStickACalibrated:=true;
    ResetStars(1,Y,PageWidth,Y+15);
    XRectFill(1,Y,PageWidth,Y+15,VisiblePageOffs,0);
  end;
  repeat StarFieldStep until JoyStickButtonDown=0;
end;

procedure InitialiseMouse(Y:integer);
var i:integer;
begin
  if not MousePresent then
  begin
    XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Mouse not found');
    PlayerInfo[player1].InputDevice:=KeyboardInput;
    PlayerInfo[player2].InputDevice:=KeyboardInput;
    WaitForEvent(Stars);
    exit;
  end;
  XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Press fire button');
  repeat
    StarFieldStep;
    i:=ButtonPressed;
  until ((i and MouseButtonMask)<>0) or (numkeypresses(kESC)>0);
  if LastKeyHit=#27 then exit;
  PlayerInfo[Player].MouseFireButton:=i;
  XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Press smartbomb button');
  repeat
    StarFieldStep;
    i:=ButtonPressed;
  until ((i and MouseButtonMask)<>0) or (numkeypresses(kESC)>0);
  if LastKeyHit=#27 then exit;
  PlayerInfo[Player].MouseSmartBombButton:=i;

  repeat StarFieldStep until ButtonDown=0;
end;

procedure InitialiseKeyboard(Y:integer);
var k:KeysType;
const KeyNameList:array[UpKey..SmartBombKey] of string[10]=
      ('Up', 'Down', 'Left', 'Right', 'Up Left', 'Up Right',
       'Down Left', 'Down Right', 'Brake', 'Fire', 'Smart Bomb');
begin
  XPrintfCenterStars(160,Y,VisiblePageOffs,BaseColor,'Press the key you want for the following:');
  for k:=UpKey to SmartBombKey do
  begin
    XPrintfCenterStars(160,Y+20,VisiblePageOffs,BaseColor,'    '+KeyNameList[k]+'    ');
    repeat
      StarFieldStep;
    until keypressed;
    if LastKeyHit=#27 then exit;
    PlayerInfo[Player].KeyArray[k]:=LastScanCode;
  end;
  ClearInputBuffers(Stars);
end;


Procedure XPutFontPbm( X,Y:word;Color:byte;ScrnOffs:word; var Bitmap ); assembler;
{As for XPutPBM, but adds the value of Color to every pixel}
var
	Plane,BMHeight:byte;
	LineInc:word;
asm
	push  ds
	cld
	mov   ax,SCREENSEG
	mov   es,ax
	mov   bx,Y
	shl   bx,1
	mov   ax,word ptr [ScrOffsetTable+bx]
	mov   bx,ScrnLogicalByteWidth
	mov   di,ScrnOffs
	add   di,ax
	mov   cx,X
	mov   dx,cx
	shr   dx,2
	add   di,dx

	lds   si,Bitmap
	lodsw
	mov   BMHeight,ah
	xor   ah,ah
	sub   bx,ax
	mov   LineInc,bx
	mov   bh,al

	and   cx,0003h

	mov   ah,11h
	shl   ah,cl
	mov   dx,SCINDEX
	mov   al,MAPMASK
	out   dx,al
	inc   dx
	mov   [Plane],4
@@PlaneLoop:
	push  di
	mov   bl,BMHeight
	mov   al,ah
	mov   dx,SCINDEX+1      {saves dx for use in RowLoop}
	out   dx,al
@@RowLoop:
	mov   cl,bh

@@MoveLoop:
	mov   dl,byte ptr [si]
	cmp   dl,0
	jz    @@zerobyte
	add   dl,byte ptr [Color]
@@zerobyte:
	mov   byte ptr es:[di],dl
	inc   si
	inc   di
	dec   cx
	jnz   @@MoveLoop

	add   di,LineInc
	dec   bl
	jnz   @@RowLoop
	pop   di
	rol   ah,1
	adc   di,0
	dec   Plane
	jnz   @@PlaneLoop
	pop   ds
end;

procedure XPrintf(X,Y,Page,Color:word;s:string);
var dx,i:integer;
begin
  dx:=0;
  for i:=1 to length(s) do
  begin
{$IFDEF DEBUG}
    if comixfont[ord(s[i])].width=0 then
    begin
      XTextMode;
      writeln('Illegal character: ',s[i],' (',ord(s[i]),') written.');
      halt;
    end else
{$ENDIF}
    XPutFontPBM(x+dx,y,Color,Page,comixfont[ord(s[i])].pic^);
    inc(dx,comixfont[ord(s[i])].width);
  end;
end;

function LengthOfComixFontString(s:string):integer;
var i,j:integer;
begin
  j:=0;
  for i:=1 to length(s) do j:=j+comixfont[ord(s[i])].width;
  LengthOfComixFontString:=j;
end;



procedure XPrintfCenter(Center,Y,Page,Color:word;s:string);
var X:integer;
begin
  X:=Center-LengthOfComixFontString(s) div 2;
  XPrintf(X,Y+1,Page,Color,s);
end;

procedure XPrintfCenterStars(Center,Y,Page,Color:word;s:string);
var X,Width:integer;
begin
  Width:=LengthOfComixFontString(s);
  X:=Center-Width div 2;
  ResetStars(X-1,Y-1,X+Width+5,Y+15);
  XRectFill(X-1,Y-1,X+Width+5,Y+15,VisiblePageOffs,0);
  XPrintf(X,Y,Page,Color,s);
end;

procedure XCorner1(X,Y,S,C1,C2:integer;Page:word);
var i,j,c:integer;
begin
  for i:=0 to (S-1) do
  for j:=0 to (S-1) do
  begin
    if i<j then c:=C1
      else if i>j then c:=C2
	else c:=(C1+C2) div 2;
    XPutPix(X+i,Y+j,Page,c);
  end;
end;

procedure XCorner2(X,Y,S,C1,C2:integer;Page:word);
var i,j,c:integer;
begin
  for i:=0 to (S-1) do
  for j:=0 to (S-1) do
  begin
    if (S-i-1)<j then c:=C1
      else if (S-i-1)>j then c:=C2
	else c:=(C1+C2) div 2;
    XPutPix(X+i,Y+j,Page,c);
  end;
end;


procedure XWindow(X1,Y1,X2,Y2,BS,Page:word);
begin
  XRectFill(X1+BS,Y1+BS,X2-BS,Y2-BS,Page,14);
  XRectFill(X1+BS,Y1,X2-BS,Y1+BS,Page,23);
  XRectFill(X1+BS,Y2-BS,X2-BS,Y2,Page,10);
  XRectFill(X1,Y1+BS,X1+BS,Y2-BS,Page,20);
  XRectFill(X2-BS,Y1+BS,X2,Y2-BS,Page,12);
  XCorner1(X1,Y1,BS,20,23,Page);
  XCorner1(X2-BS,Y2-BS,BS,10,12,Page);
  XCorner2(X1,Y2-BS,BS,10,20,Page);
  XCorner2(X2-BS,Y1,BS,12,23,Page);
  TextWindowX:=X1;
  TextWindowY:=Y1;
end;

procedure DrawBox(X1,Y1,X2,Y2:integer;Page:word);
begin
    XLine(X1,Y1,X2,Y1,Page,10);
    XLine(X1,Y2,X2,Y2,Page,23);
    XLine(X1,Y1+1,X1,Y2,Page,10);
    XLine(X2,Y1+1,X2,Y2,Page,23);
end;

procedure XText(X,Y:integer;Box:boolean;Page:word;s:string);
var i:integer;
begin
  for i:=1 to length(s) do
  begin
    XPutPBM(X+8*(i-1),Y,Page,font[fontmap[s[i]]]^);
  end;
  if box then DrawBox(X-2,Y-1,X+length(s)*8+2,Y+14,Page);
end;

procedure XTextCenter(X,Y:integer;Box:boolean;Page:word;s:string);
begin
  XText(X-length(s)*4,Y,Box,Page,s);
end;

procedure MyDelay(Msec:word);
var time:longint;
begin
  time:=TimeCount;
  repeat until TimeCount>=time+Msec/1000*Ticks;
end;

procedure SetXMode;
var i:integer;
begin
  i:=XSetMode(XMode320x240,PageWidth);
  if i<>PageWidth then
  begin
    XTextMode;
    writeln('Mode set error. Page Width ',i,' returned.');
    WaitForEvent(NoStars);
    halt;
  end;
  XSetSplitScreen(SplitScreenLine); {gives a 241-line screen!}
  i:=XSetDoubleBuffer(PageHeight);
  if i<>PageHeight then
  begin
    XTextMode;
    writeln(PageHeight,' asked for, ',i,' returned. MaxScrollY: ',maxscrolly,' ',scrnlogicalheight);
    WaitForEvent(NoStars);
    halt;
  end;

end;

Function ButtonPressed:integer;
  {checks for mouse or joystick button press}
var button,but,count,hcur,vcur:integer;
 begin
  Button:=0;
  if MousePresent then
  begin
    but:=0;
    mbutpress(but,count,hcur,vcur);
    if count>0 then Button:=Button or 1;
    but:=1;
    mbutpress(but,count,hcur,vcur);
    if count>0 then Button:=Button or 2;
  end;
  if JoyAPresent then
  begin
    Button:=Button or JoyStickButtonPressed;
  end;
  ButtonPressed:=Button;
end;

Function ButtonDown:integer;
  {checks for mouse or joystick button press}
var but,i,j:integer;
begin
  but:=0;
  if MousePresent then
  begin
    mpos(but,i,j);
  end;
  if JoyAPresent then
  begin
    But:=But or JoyStickButtonDown;
  end;
  ButtonDown:=But;
end;


function DelayOrEvent(timeinticks:longint):boolean;
  {returns false if timed out, true if event occurred}
var temp:longint;
    i:integer;
begin
  temp:=timecount+timeinticks;
  mbutpress(i,i,i,i);
  repeat until keypressed or (ButtonPressed<>0) or (timecount>temp);
  ClearKeyPresses;
  DelayOrEvent:=(timecount<=temp);
end;

procedure ClearInputBuffers(Stars:boolean);
begin
  repeat
    if Stars then StarFieldStep;
  until ((ButtonPressed=0) and (ButtonDown=0));
  ClearKeyPresses;
end;

procedure WaitForEvent(Stars:boolean);
begin
  ClearInputBuffers(Stars);
  repeat
    if Stars then StarFieldStep;
  until keypressed or (ButtonPressed<>0);
  ClearInputBuffers(Stars);
end;

function YesNoStars(default:char):char;
var ch:char;
begin
  ch:=#0;
  clearkeypresses;
  repeat
    StarFieldStep;
    if numkeypresses(kY)>0 then ch:='Y';
    if numkeypresses(kN)>0 then ch:='N';
    if numkeypresses(kESC)>0 then ch:='N';
    if numkeypresses(kENTER)>0 then ch:=default;
  until ch<>#0;
  yesnoStars:=ch;
  ClearInputBuffers(Stars);
end;

procedure SetXModeNoSplitScreen;
begin
  if XSetMode(XMode320x240,PageWidth)<>PageWidth then
  begin
    XTextMode;
    writeln('X mode set error. Erk!');
    WaitForEvent(NoStars);
    halt(14);
  end;
  VisiblePageX:=0;
  VisiblePageY:=0;
end;




procedure TitlePage;
var p:pointer;
    f:file;
type arr=array[0..769] of byte;
var pal2:^arr;
    a:byte;
    b:integer;
begin
  SetXModeNoSplitScreen;
  XPutPalRaw(blankpalette,256,0);
  CheckExistence('title0.pbm');
  assign(f,'title0.pbm');
  reset(f,1);
  getmem(p,filesize(f));
{$IFDEF TESTMEM}
      writeln(memfile,'Title allocation: ',filesize(f),' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
  blockread(f,p^,filesize(f));
  XPutPBM(80,95,Page0Offs,p^);
  freemem(p,filesize(f));
{$IFDEF TESTMEM}
      writeln(memfile,'Title deallocation: ',filesize(f),' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
  close(f);
  PlaySound(gatesound);
  MyDelay(100);
  XPutPalStruc(palette);
  ClearInputBuffers(NoStars);
  DelayOrEvent(3*longint(ticks) div 2);
  new(pal2);  {fade out}
  pal2^[0]:=palette[0];
  pal2^[1]:=palette[1];
  for a:=64 downto 0 do
  begin
    for b:=2 to 769 do pal2^[b]:=(palette[b]*a) shr 6;
    xputpalstruc(pal2^);
  end;
  CheckExistence('title.pbm');
  assign(f,'title.pbm');
  reset(f,1);
  getmem(p,filesize(f) div 2);
  blockread(f,p^,filesize(f) div 2);
  XPutPBM(0,0,Page0Offs,p^);
  blockread(f,p^,filesize(f) div 2);
  XPutPBM(0,120,Page0Offs,p^);
  freemem(p,filesize(f) div 2);
  close(f);
  PlaySound(gatesound);
  MyDelay(100);
  XPutPalStruc(titlepalette);
  ClearInputBuffers(NoStars);
  DelayOrEvent(longint(ticks)*3);
  pal2^[0]:=0;
  pal2^[1]:=255;
  for a:=64 downto 0 do
  begin
    for b:=2 to 769 do pal2^[b]:=(titlepalette[b]*a) shr 6;
    xputpalstruc(pal2^);
  end;
  dispose(pal2);  {dispose palette memory}
end;

procedure MenuScreenSetup;
var f:file;
begin
  Xputpalraw(blankpalette,256,0);
  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page0Offs,0);
{$IFDEF STARTPIC}
  CheckExistence('startpic.pbm');
  assign(f,'startpic.pbm');
  reset(f,1);
  getmem(StartPicPBM,filesize(f));
{$IFDEF TESTMEM}
      writeln(memfile,'Title allocation: ',filesize(f),' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
  blockread(f,StartPicPBM^,filesize(f));
  XPutPBM(0,20,VisiblePageOffs,StartPicPBM^);
  freemem(StartPicPBM,filesize(f));
{$IFDEF TESTMEM}
      writeln(memfile,'Title deallocation: ',filesize(f),' bytes. ',memavail,' free, max block ',maxavail);
{$ENDIF}
  close(f);
{$ENDIF}
  InitStars;
  XPutPalStruc(palette);
  ClearInputBuffers(true);
end;

function GetString(X,Y,MaxLength,MaxScreenLength,Page,Color:word):string;
var s:string[50];
begin
  s:='';
  XPrintf(X,Y,Page,Color,s+'_');
  ClearInputBuffers(Stars);
  repeat
    repeat
      StarFieldStep;
    until keypressed;
    if numkeypresses(kDEL)+numkeypresses(kKEYPADDEL)+numkeypresses(kBACKSPACE)>0 then
    begin
      if length(s)>0 then s:=copy(s,1,length(s)-1);
    end
    else
    if (LastKeyHit<>#13) and (LastKeyHit<>#27) then
      if (length(s)<MaxLength) and
	 (LengthOfComixFontString(s)<MaxScreenLength)
	 and (comixfont[ord(LastKeyHit)].width>0)
	 then s:=s+LastKeyHit;
    ResetStars(X,Y,X+165,Y+14);
    XRectFill(X,Y,X+165,Y+14,Page,0);
    if (length(s)<MaxLength) and
       (LengthOfComixFontString(s)<MaxScreenLength) then
      XPrintf(X,Y,Page,Color,s+'_')
    else
      XPrintf(X,Y,Page,Color,s)
  until LastKeyHit=#13;
  XRectFill(X,Y,X+165,Y+14,Page,0);
  GetString:=s;
end;

function GetFileName(X,Y,Page,Color:word):string;
type filelistnodeptr=^filelistnode;
     filelistnode=record
		    next:filelistnodeptr;
		    prev:filelistnodeptr;
		    name:string[12];
		  end;
var first,work:filelistnodeptr;
    s:SearchRec;  {DOS struct}
    i:integer;
begin
  new(first);
  first^.next:=first;
  first^.prev:=first;
  findfirst('*.dmo',Archive or ReadOnly,s);
  if DOSError<>0 then
  begin
    GetFileName:=#27;
    exit;
  end;
  first^.name:=s.name;
  work:=first;
  findnext(s);
  while DOSError=0 do
  begin
    new(work^.next);
    work^.next^.prev:=work;
    work^.next^.next:=first;
    first^.prev:=work^.next;
    work^.next^.name:=s.name;
    work:=work^.next;
    findnext(s);
  end;
  work:=first;
  repeat
    ResetStars(X,Y,X+165,Y+14);
    XRectFill(X,Y,X+165,Y+14,Page,0);
    XPrintf(X,Y,Page,Color,work^.name);
    repeat
      StarFieldStep;
      i:=ButtonPressed;
    until (keypressed) or (i<>0);
    if (numkeypresses(kLARROW)+numkeypresses(kUARROW)+
      +numkeypresses(kKEYPAD8)+numkeypresses(kKEYPAD4))>0 then work:=work^.prev;
    if (numkeypresses(kRARROW)+numkeypresses(kDARROW)+
      +numkeypresses(kKEYPAD2)+numkeypresses(kKEYPAD6))>0 then work:=work^.prev;
    if ((i and 1)<>0) then work:=work^.next;
  until (numkeypresses(kENTER)>0) or (numkeypresses(kESC)>0) or ((i and 2)<>0);
  XRectFill(X,Y,X+165,Y+14,Page,0);
  work:=first;
  while work^.next<>first do
  begin
    work:=work^.next;
    dispose(work^.prev);
  end;
  dispose(work);
  if (LastKeyHit=#27) then GetFileName:=''
    else GetFileName:=work^.name;
end;


procedure HiScores(seconds:word);
var scorefile:file of scoretype;
    sc:array[0..MaxDiffLevel-1,1..10] of scoretype;
    i,j,k:integer;
    s:string[20];
    pl:PlayerType;

begin
  XSetRGB(HighLightColor+1,55,55,55);
  XSetRGB(HighLightColor+2,25,25,25);
  {$I-}
  assign(scorefile,'xquest.scr');
  reset(scorefile);
  for i:=0 to MaxDiffLevel-1 do for j:=1 to 10 do read(scorefile,sc[i,j]);
  close(scorefile);
  {$I+}
  if ioresult<>0 then
  begin
    sc[1,1].score:=0;
    sc[1,1].level:=1;
    sc[1,1].name:='XQuest';
    {$I-}
    assign(scorefile,'xquest.scr');
    rewrite(scorefile);
    for i:=0 to MaxDiffLevel-1 do
    for j:=1 to 10 do
    begin
      sc[i,j]:=sc[1,1];
      write(scorefile,sc[1,1]);
    end;
    close(scorefile);
    {$I+}
    if ioresult<>0 then begin end;       {clear ioerror}
  end;
  for pl:=Player1 to Player2 do
  with PlayerInfo[pl] do
  if (pl=Player1) or (GameMode=TwoPlayer) then
  with GameInfo[pl] do
  begin
    ClearInputBuffers(true);
    ResetStars(10,80,310,239);
    XRectFill(10,80,310,239,VisiblePageOffs,0);
    k:=0;
    for i:=1 to 10 do
    begin
      if score>sc[DiffLevel,i].score then
      begin
	k:=i;
	for j:=10 downto (i+1) do sc[DiffLevel,j]:=sc[DiffLevel,j-1];
	sc[DiffLevel,i].score:=score;
	sc[DiffLevel,i].level:=TotalLevel;
	score:=0;
      end
      else XPrintf(20,86+i*14,VisiblePageOffs,BaseColor,sc[DiffLevel,i].name);
      str(sc[DiffLevel,i].level,s);
      XPrintf(300-LengthOfComixFontString(s),86+i*14,VisiblePageOffs,BaseColor,s);
      str(sc[DiffLevel,i].score,s);
      XPrintf(270-LengthOfComixFontString(s),86+i*14,VisiblePageOffs,BaseColor,s);
    end;
    if k<>0 then
    begin
      if k=1 then
      begin
        if GameMode=TwoPlayer then
        begin
	  if Pl=Player1 then XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'PLAYER 1: A NEW HIGH SCORE!!!')
	   else XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'PLAYER 2: A NEW HIGH SCORE!!!');
        end
          else XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'A NEW HIGH SCORE!!!');
        PlaySound(woohoo);
      end
      else
      begin
        if GameMode=TwoPlayer then
        begin
	  if Pl=Player1 then XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'PLAYER 1: A TOP TEN SCORE!!!')
	   else XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'PLAYER 2: A TOP TEN SCORE!!!');
        end
          else XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'A TOP TEN SCORE!!!');
        PlaySound(allright);
      end;
      PlaySound(applause);
      for i:=1 to 20 do StarFieldStep;
      if MaxSoundEffects>2 then PlaySound(applause);
      for i:=1 to 20 do StarFieldStep;
      if MaxSoundEffects>3 then PlaySound(applause);
      sc[DiffLevel,k].name:=GetString(20,86+k*14,20,160,VisiblePageOffs,BaseColor);
      if sc[DiffLevel,k].name='' then sc[DiffLevel,k].name:='Anonymous';
      XPrintf(20,86+k*14,VisiblePageOffs,BaseColor,sc[DiffLevel,k].name);
      {$I-}
      rewrite(scorefile);
      for i:=0 to MaxDiffLevel-1 do
      for j:=1 to 10 do
	write(scorefile,sc[i,j]);
      close(scorefile);
      {$I+}
    end else XPrintfCenter(160,80,VisiblePageOffs,HighLightColor,'HALL OF FAME ('+Diffname2[difflevel]+')');
    k:=0;
    repeat
      StarFieldStep;
      inc(k);
    until (ButtonPressed<>0) or keypressed or (k>(seconds*FrameRate));
  end;
  ClearInputBuffers(true);
end;

procedure ShowHelp;
var i,j,frame:integer;
    s:string[20];
begin
  ResetStars(0,90,310,230);
  XRectFill(0,90,311,230,VisiblePageOffs,0);
  XprintfCenter(160,95,VisiblePageOffs,BaseColor,'Use the mouse or joystick to move');
  XprintfCenter(160,110,VisiblePageOffs,BaseColor,'the ship     around. Collect the');
  XprintfCenter(160,125,VisiblePageOffs,BaseColor,'gems    and avoid the mines    .');
  XprintfCenter(160,140,VisiblePageOffs,BaseColor,'Button 1 fires your bullets, while');
  XprintfCenter(160,155,VisiblePageOffs,BaseColor,'button 2 or the spacebar activates');
  XprintfCenter(160,170,VisiblePageOffs,BaseColor,'a SmartBomb if you have any.');
  XprintfCenter(160,215,VisiblePageOffs,BaseColor,'Hit a button or key to continue.');
  XPutCBitmap(118,111,VisiblePageOffs,ship.pic[0]^);
  XPutCBitmap(95,126,VisiblePageOffs,objects.pic[crys]^);
  XPutCBitmap(240,126,VisiblePageOffs,objects.pic[mine]^);
  WaitForEvent(Stars);
  if LastKeyHit=#27 then exit;
  ResetStars(0,90,310,230);
  XRectFill(0,90,311,230,VisiblePageOffs,0);
  XprintfCenter(160,95,VisiblePageOffs,BaseColor,'Collect extra SmartBombs     and look');
  XprintfCenter(160,110,VisiblePageOffs,BaseColor,'out for PowerCharges     which will');
  XprintfCenter(160,125,VisiblePageOffs,BaseColor,'give you special powerups! Watch out,');
  XprintfCenter(160,140,VisiblePageOffs,BaseColor,'though, PowerCharges are fragile and');
  XprintfCenter(160,155,VisiblePageOffs,BaseColor,'will shatter if you shoot them.');
  XprintfCenter(160,170,VisiblePageOffs,BaseColor,'If the going gets too tough hit ''p''');
  XprintfCenter(160,185,VisiblePageOffs,BaseColor,'to pause the game.');
  XprintfCenter(160,215,VisiblePageOffs,BaseColor,'Hit a button or key to continue.');
  XPutCBitmap(207,96,VisiblePageOffs,objects.pic[smart]^);
  frame:=0;
  repeat
    with enemykind[0] do
    begin
      inc(frame,framespeed);
      if frame>=((numframes+1) shl 8) then frame:=0;
		 { plot frames 0.. numframes }
      ResetStars(187,110,187+width,110+height);
      XPutCBitmap(187,110,VisiblePageOffs,pic[frame shr 8]^);
      StarFieldStep;
      XRectFill(187,110,187+width,110+height,VisiblePageOffs,0);
    end;
  until (ButtonPressed<>0) or keypressed;
  if LastKeyHit=#27 then exit;
  ResetStars(0,90,310,230);
  XRectFill(0,90,311,230,VisiblePageOffs,0);
  XPrintfCenter(160,95,VisiblePageOffs,BaseColor,'There are many types of nasties just');
  XPrintfCenter(160,110,VisiblePageOffs,BaseColor,'waiting to do some damage...');
  i:=2;j:=0;frame:=0;
  repeat
    inc(j,2);
    with enemykind[i] do
    begin
      ResetStars(j,140,j+width,140+height);
      XPutCBitmap(j,140,VisiblePageOffs,pic[frame shr 8]^);
      StarFieldStep;
      inc(frame,framespeed);
      if frame>=((numframes+1) shl 8) then frame:=0;
		 { plot frames 0.. numframes }
      XRectFill(j,140,j+width,140+height,VisiblePageOffs,0);
      if j>310 then
      begin
	j:=0;
	inc(i);
	if i>(MaxEnemyKinds) then i:=2;
	if i=6 then inc(i);
	if i=14 then inc(i);   {skip tribble and hibernating one}
	frame:=0;
      end;
      if (j=120) then
      begin
	ResetStars(80,170,240,210);
	XRectFill(80,170,240,210,VisiblePageOffs,0);
	XPrintfCenter(160,170,VisiblePageOffs,BaseColor,enemyname[i]);
	str(enemykind[i].score,s);
	s:=s+' points';
	XPrintfCenter(160,195,VisiblePageOffs,BaseColor,s);
      end;
      if j=200 then
      begin
	ResetStars(80,170,240,210);
	XRectFill(80,170,240,210,VisiblePageOffs,0);
      end;
    end;
  until (ButtonPressed<>0) or (keypressed);
  if LastKeyHit=#27 then exit;
  ResetStars(0,90,310,230);
  XRectFill(0,90,311,230,VisiblePageOffs,0);
  XprintfCenter(160,95,VisiblePageOffs,BaseColor,'When all the gems have been');
  XprintfCenter(160,110,VisiblePageOffs,BaseColor,'collected the gate will open and you');
  XprintfCenter(160,125,VisiblePageOffs,BaseColor,'can go on to the next level. If you');
  XprintfCenter(160,140,VisiblePageOffs,BaseColor,'complete the level in less than the');
  XprintfCenter(160,155,VisiblePageOffs,BaseColor,'par time you will be awarded a time');
  XprintfCenter(160,170,VisiblePageOffs,BaseColor,'bonus. If your score gets high enough');
  XprintfCenter(160,185,VisiblePageOffs,BaseColor,'you may also get extra lives.');
  XprintfCenter(160,215,VisiblePageOffs,BaseColor,'Hit a button or key to continue.');
  WaitForEvent(Stars);
  if LastKeyHit=#27 then exit;
  ResetStars(0,90,310,230);
  XRectFill(0,90,311,230,VisiblePageOffs,0);
  XprintfCenter(160,95,VisiblePageOffs,BaseColor,'The status bar at the bottom of the');
  XprintfCenter(160,110,VisiblePageOffs,BaseColor,'screen shows your score, the number');
  XprintfCenter(160,125,VisiblePageOffs,BaseColor,'of lives and smartbombs you have left,');
  XprintfCenter(160,140,VisiblePageOffs,BaseColor,'the number of gems left, and what');
  XprintfCenter(160,155,VisiblePageOffs,BaseColor,'powerups you currently possess.');
  XprintfCenter(160,185,VisiblePageOffs,BaseColor,'Now go and get those gems!');
  XprintfCenter(160,215,VisiblePageOffs,BaseColor,'Hit a button or key to return to the menu.');
  WaitForEvent(Stars);
end;

procedure XDrawSlideBar(X,Y,Page,Color,BColor,Min,Max:integer; var value:integer);
var i,j:integer;
begin
{  ResetStars(X-1,Y-1,X+101,Y+12);  takes too long}
  XRectFill(X-1,Y-1,X+101,Y+12,Page,BColor);
  XRectFill(X+round(100.0*(value-Min)/(Max-Min)),Y,X+100,Y+11,Page,0);
  XRectFill(X,Y,X+round(100.0*(value-Min)/(Max-Min)),Y+11,Page,Color);
end;


function XMenu(var Items:MenuListType;InitialSelection:integer;TimeOut:word):integer;
  {timeout in second}
type PalType=record
	       r,g,b:word;
	     end;

Const MaxColorCycle=13;
      ColorCyclePal:array[1..MaxColorCycle] of PalType=
		    ((r:45;g:45;b:45),(r:40;g:40;b:48),
		     (r:35;g:35;b:51),(r:30;g:30;b:54),
		     (r:25;g:25;b:57),(r:20;g:20;b:60),
		     (r:15;g:15;b:63),(r:20;g:20;b:60),
		     (r:25;g:25;b:57),(r:30;g:30;b:54),
		     (r:35;g:35;b:51),(r:40;g:40;b:48),
		     (r:45;g:45;b:45));
var selected:byte;
    i,k,but,mx,my:integer;
    ColorCycle,FrameCount:word;
    MenuY:word;


  procedure DisplayMenuItem(i:integer;highlight:boolean);
  var YPos:integer;
  begin
    YPos:=MenuY+i*20;
    with Items.s[i] do
    begin
      if not highlight then
      begin
	if mtype<>slidebar then
	  XPrintfCenter(160,YPos,VisiblePageOffs,BaseColor,name)
	else
	begin
	  XPrintf(160-LengthOfComixFontString(name),Ypos,VisiblePageOffs,BaseColor,name);
	  XDrawSlideBar(170,YPos,VisiblePageOffs,22,27,min,max,value);
	end;
      end
      else
      begin
	if mtype<>slidebar then
	begin
	  XPrintfCenter(160,YPos,VisiblePageOffs,HighLightColor,name);
	end
	else
	begin
	  XPrintf(160-LengthOfComixFontString(name),YPos,VisiblePageOffs,HighLightColor,name);
	  XDrawSlideBar(170,YPos,VisiblePageOffs,22,27,min,max,value);
	end;
      end;
    end;
  end;


begin
  ClearInputBuffers(Stars);
  TimeOut:=TimeOut*FrameRate;
  MenuY:=(7-Items.num)*10+80;
  ResetStars(0,80,320,240);
  XRectFill(0,80,320,240,VisiblePageOffs,0);
  if InitialSelection>0 then Selected:=InitialSelection
    else Selected:=0;
  FrameCount:=0;
  mx:=0;my:=0;
  for i:=0 to (Items.num-1) do
  begin
    DisplaymenuItem(i,i=selected);
    with items.s[i] do
    if (i=selected) and (mtype=slidebar) then
	mx:=round((value-Min)*(2*64)/(Max-Min)-64)
  end;
  repeat
    if (my<-40) or ((numkeypresses(kUARROW)+numkeypresses(kKEYPAD8))>0) then
    begin
      if selected>0 then
      begin
	ColorCycle:=MaxColorCycle*3 div 2;
	DisplayMenuItem(selected,false);
	dec(selected);
	PlaySound(menuclick);
      end;
      with items.s[selected] do
      if mtype=slidebar then
	mx:=round((value-Min)*(2*64)/(Max-Min)-64)
      else mx:=0;
      my:=0;
    end;
    if (my>40) or ((numkeypresses(kDARROW)+numkeypresses(kKEYPAD2))>0) then
    begin
      if selected<(Items.num-1) then
      begin
	ColorCycle:=MaxColorCycle*3 div 2;
	DisplayMenuItem(selected,false);
	inc(selected);
       PlaySound(menuclick);
      end;
      with items.s[selected] do
      if mtype=slidebar then
	mx:=round((value-Min)*(2*64)/(Max-Min)-64)
      else mx:=0;
      my:=0;
    end;
    DisplayMenuItem(selected,true);
    repeat
      inc(ColorCycle);
      if ColorCycle>=MaxColorCycle*3 then ColorCycle:=1;
      XSetRGB(HighLightColor+1,ColorCyclePal[ColorCycle div 3 +1].r,
			       ColorCyclePal[ColorCycle div 3 +1].g,
			       ColorCyclePal[ColorCycle div 3 +1].b);
      XSetRGB(HighLightColor+2,ColorCyclePal[ColorCycle div 3 +1].r div 2,
			       ColorCyclePal[ColorCycle div 3 +1].g div 2,
			       ColorCyclePal[ColorCycle div 3 +1].b div 2);
      StarFieldStep;
      inc(FrameCount);
      InputMovement(mx,my,true,true,true,false);
      but:=ButtonDown;
    until (But<>0) or keypressed or (my<-40) or (my>40) or (FrameCount>TimeOut);
    if numkeypresses(kJ)>0 then InitialiseJoyStick(220);
    with Items.s[selected] do
    if (mtype=slidebar) then
    begin
      if (but and (1 or JoyStickAButton1))<>0 then
      begin
	value:=round((mx+64)*(Max-Min)/(2*64) + Min);
	but:=0;
      end;
      if (numkeypresses(kLARROW)+numkeypresses(kKEYPAD4))>0 then
        dec(value,round((Max-Min)/50)+1)
      else if (numkeypresses(kRARROW)+numkeypresses(kKEYPAD6))>0 then
        inc(value,round((Max-Min)/50)+1);
      if value<min then value:=min;
      if value>max then value:=max;
      if mx>64 then mx:=64;if mx<-64 then mx:=-64;
    end;
    if (but and (JoyStickAButton1 or JoyStickAButton2))<>0 then
      repeat StarFieldStep until JoyStickButtonDown=0
    else but:=ButtonPressed;
    i:=numkeypresses(kENTER);
  until (but and (2 or JoyStickAButton2)<>0) or
	((but and (1 or JoyStickAButton1)<>0) and (Items.s[selected].mtype=item))
	or (numkeypresses(kESC)>0) or (i>0) or (FrameCount>TimeOut);
  if (but and (1 or JoyStickAButton1)<>0) or (i>0) then XMenu:=Selected
    else if (FrameCount>TimeOut) then XMenu:=MenuTimeOut
      else XMenu:=NoItemSelected;
  DisplayMenuItem(selected,false);
end;

procedure EndScreen;
var a,b:integer;
    f:file;
    delaycount:longint;
begin
  XWindow(110+VisiblePageX,94+VisiblePageY,210+VisiblePageX,126+VisiblePageY,2,VisiblePageOffs);
  XText(TextWindowX+15,TextWindowY+9,False,VisiblePageOffs,'GAME OVER');
  DelayOrEvent(2*ticks);
  if demomode then
  begin
    GameInfo[Player1].Score:=-10;
    GameInfo[Player2].Score:=-10;   {so that you don't get hiscores!}
    demomode:=false;
  end;
  if recording then
  begin
    blockwrite(demofile,demo^,demoptr*5);
    close(demofile);
    recording:=false;
    XWindow(90+VisiblePageX,90+VisiblePageY,230+VisiblePageX,130+VisiblePageY,2,VisiblePageOffs);
    XText(TextWindowX+15,TextWindowY+13,False,VisiblePageOffs,'DEMO RECORDED');
    DelayOrEvent(2*Ticks);
  end;
  mstatus(a,b);  {reset mouse}
  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page0Offs,0);
  XRectFill(0,0,ScrnLogicalPixelWidth,ScrnLogicalHeight,Page1Offs,0);
		 {clear screen}
  SetXModeNoSplitScreen;
  MenuScreenSetup;
  HiScores(10);  {10 second timeout}
end;

end.
