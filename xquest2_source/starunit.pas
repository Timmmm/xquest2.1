{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*            Copyright (C) 1994 M.Mackey. All rights reserved.              *}
{*                                                                           *}
{*                             Starfield Unit                                *}
{*                                                                           *}
{*****************************************************************************}


{$M 16384,0,655360}
unit starunit;

(* Requires the VGA to be set to mode X, page 0000h
   Does not change the video mode *)

interface

uses crt,xqvars,xlib;

const StarsInitialised:boolean=false;

procedure initstars;
procedure starfieldcont;
procedure starfieldstep;
procedure starfieldstepNoVSync;
procedure resetstars(X1,Y1,X2,Y2:integer);
procedure starsfinished;

implementation

const MaxStars=400;         {Decrease for slower computers}
      speed:word=128;       {speed of movement thru starfield}


 { basic screen size stuff used for star animation. }
const XWIDTH = 320;
const YWIDTH = 240;

const XCENTER = ( XWIDTH div 2 );
const YCENTER = ( YWIDTH div 2 );

type STARtype=record {12 bytes}
		x,y,z:integer; {The x, y and z coordinates}
		xz,yz:integer; { screen coords}
		c:integer;
	      end;
     stararraytype=array[1..maxstars] of startype;

var star:^stararraytype;
    i,j,NumStars:integer;
    ch:char;

procedure initstar(i:integer);  {initialise stars at random positions}
begin
  with star^[i] do
  begin
    x := longint(-5000)+random(10000);
    y := longint(-5000)+random(10000);             {at rear}
    z := random(12000)+256;
    xz:=1;
    yz:=1;
    c:=0;
  end;
end;

procedure newstar(i:integer);   {create new star at front of starfield}
begin
  with star^[i] do
  begin
    x := longint(-8191)+random(16383);
    y := longint(-8191)+random(16383);
    z := random(1256)+14500;
    xz:=1;
    yz:=1;
    c:=0;
  end;
end;


procedure update(var star:startype;i:integer);assembler;
{
;  This procedure erases the star, recalculates its position and redraws
;  it. NOTE: star passed by reference
}
asm
	les     si,star
	cmp     word ptr es:[si+startype.xz],0
	jl      @@GetNewStar
	cmp     word ptr es:[si+startype.yz],0
	jl      @@GetNewStar
	cmp     word ptr es:[si+startype.xz],xwidth
	jg      @@GetNewStar
	cmp     word ptr es:[si+startype.yz],ywidth
	jg      @@GetNewStar
	cmp     word ptr es:[si+startype.c],0
	je      @@DontErase
	push    es
	push    si
	push    es:[si+startype.xz]     { push x}
	push    es:[si+startype.yz]     { push y}
	push    VisiblePageOffs
	push    0F00h
	call    Xputpix         { erase old star position: speed up}
	pop     si
	pop     es

@@DontErase:
	mov     ax,speed
	sub     word ptr es:[si+startype.z],ax  { move all stars towards viewer}
	cmp     es:[si+startype.z],257          { z<256 gets a divide
						 by zero exception later on}
	jge     @@Z_OK

@@GetNewstar:
	push    i
	call    newstar
	jmp     @@finished

@@Z_OK:
  { z is between 256 and 16012}

	mov     dx,0
	mov     ax,es:[si+startype.x]
	cmp     ax,0
	jge     @@SignextendOK
	not     dx
@@SignExtendOK:   {x sign extended to dx:ax. Kludgy!}

	mov     cx,es:[si+startype.z]
	shr     cx,8            { get hi(z) in cx}
	idiv    cx              { ax = x div hi(z) (dx:ax div cx)}
	add     ax,xcenter      { xz = (x div hi(z)) + xcenter}
				{  (perspective projection)}
	mov     es:[si+startype.xz],ax

	mov     dx,0
	mov     ax,es:[si+startype.y]
	cmp     ax,0
	jge     @@SignextendOK2
	not     dx
@@SignExtendOK2:   {y sign extended to dx:ax. Kludgy!}

	idiv    cx
	add     ax,ycenter
	mov     es:[si+startype.yz],ax  { yz = (y div hi(z)) + ycenter}
					{ (perspective projection)}

{ check screen bounds}
	cmp     word ptr es:[si+startype.xz],0
	jl      @@Finished
	cmp     word ptr es:[si+startype.yz],0
	jl      @@Finished
	cmp     word ptr es:[si+startype.xz],xwidth
	jg      @@Finished
	cmp     word ptr es:[si+startype.yz],ywidth
	jg      @@Finished

{ draw star in grayscale based on z distance}
	mov     dx,31
	mov     cx,es:[si+startype.z]
	shr     cx,9            { cx = z shr 9  ( ranges from 0 to 31)}
	sub     dx,cx           { dx = 31- (z shr 9)}
				{    i.e colour between 0 and 31}
				{ (VGA colours 0-31 are a grayscale for this palette)}

	mov     es:[si+startype.c],dx
				{store colour for erasing}

(*
	mov  ax,ScrnLogicalByteWidth
	mul  es:[si+startype.yz]
	mov  di,es:[si+startype.xz]
	shr  di,2
	add  di,ax
	add  di,VisiblePageOffs
*)

	mov  di,es:[si+startype.xz]
	shr  di,2
	mov  ax,es:[si+startype.yz]
	mul  ScrnLogicalByteWidth
	add  di,ax     {Assumes Page 0000h}


	mov  ah,byte ptr es:[si+startype.xz]            {optimise!}
	and  ah,011b
	mov  cl,ah                              {saves plane}
	mov  al,READMAP
	mov  dx,GCINDEX
	out  dx,ax
	push ds

	mov  ax,SCREENSEG
	mov  ds,ax
	mov  al,ds:[di]
	cmp  al,0
	jz   @@draw

	pop  ds
	mov  word ptr es:[si+startype.c],0
	jmp  @@finished
@@draw:
	mov  bx,word ptr es:[si+startype.c]
	mov  ax,0100h + MAPMASK
	shl  ah,cl
	mov  dx,SCINDEX
	out  dx,ax

	mov  ds:[di],bl
	pop  ds

@@Finished:
end;

procedure testspeed;  {gives an initial estimate of how many stars
			we can handle}
var i,count:word;
    temp:longint;
begin
  temp:=timecount;
  timecount:=0;
  count:=0;
  repeat
    for i:=1 to 100 do
    begin
      update(star^[1],1);
{      ch:=readkey;         }
    end;
    inc(count);
  until timecount>Ticks div 2;
  numstars:=round(count/202 *300); {dunno where this formula came from :)}
  if numstars>maxstars then numstars:=maxstars;
  timecount:=temp;
end;

procedure initstars;
begin
  if StarsInitialised then exit;
  new(star);
  for i:=1 to maxstars do initstar(i);    {initialise stars}
{  for i:=100 to 150 do
  for j:=100 to 150 do
    XPutpix(i,j,VisiblePageOffs,64);}
  testspeed;
  StarsInitialised:=true;
end;

procedure starfieldcont;
var count:word;
begin
  repeat
    timecount:=0;
    count:=0;
    repeat
      for i:=1 to numstars do update(star^[i],i);  {update star positions}
      inc(count);
      asm  {synch to vertical retrace}
	  mov   dx,03dah
  @@WaitVS:
	  in    al,dx
	  test  al,08h
	  jz    @@WaitVS
      end;

    until timecount>=Ticks div 9;
    if count<7 then   {reduce star count if not keeping up with vertical retrace}
    begin
      dec(numstars,20);
      for i:= (numstars+1) to (numstars+20) do
	with star^[i] do
	begin
	  if (xz>0) and (xz<XWidth) and (yz>0) and (Yz<YWidth) then
	    Xputpix(xz,yz,VisiblePageOffs,0);
	end;
    end;

  until keypressed;
{  close(f);}
end;

procedure starfieldstep;
var i:word;
begin
  for i:=1 to numstars do update(star^[i],i);  {update star positions}
  asm  {synch to vertical retrace}
	  mov   dx,03dah
  @@WaitVS:
	  in    al,dx
	  test  al,08h
	  jz    @@WaitVS
  end;
end;

procedure starfieldstepNoVSync;
var count:word;
begin
  for i:=1 to numstars do update(star^[i],i);  {update star positions}
end;

procedure resetstars(X1,Y1,X2,Y2:integer);
{clears stars from a rectangular region of the screen}
{assumes screen region is cleared before starfield is called again}
begin
  for i:= 1 to maxstars do
  with star^[i] do
  begin
    if (xz>X1) and (xz<X2) and (yz>Y1) and (yz<Y2) then
    star^[i].c:=0;
  end;
end;

procedure starsfinished;
begin
  if not StarsInitialised then exit;
  dispose(star);
  StarsInitialised:=false;
end;

end.



