{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*            Copyright (C) 1994 M.Mackey. All rights reserved.              *}
{*                                                                           *}
{*                              Mouse Unit                                   *}
{*                                                                           *}
{*****************************************************************************}


unit mouse;
interface
uses dos,crt;
type currarray=array[0..31] of word;
var recpack:registers;
const MouseButton1=$01;
      MouseButton2=$02;
      MouseButton3=$04;
      MouseButtonMask=$7;

const MousePresent:boolean=false;
{$R-}
procedure mstatus(var mstat,nbuttons:integer);
procedure mshow;
procedure mhide;
procedure mpos(var mbt,mx,my:integer);
procedure mput(mx,my:integer);
procedure mbutpress(var button,count,hcur,vcur:integer);
procedure mbutrelease(var button,count,hcur,vcur:integer);
procedure mhlimit(minpos,maxpos:integer);
procedure mvlimit(minpos,maxpos:integer);
procedure mgraphcursor(xhot,yhot:integer;var cursor:currarray);
procedure mtextcursor(select,screen,cursor:integer);
procedure mmotion(var hcount,vcount:integer);
procedure mspeed(hor,ver:integer);
procedure mwindowhide(x1,y1,x2,y2:integer);
procedure mspeedthreshold(speed:integer);

implementation

var i,j:integer;

procedure mstatus(var mstat,nbuttons:integer);
begin
  recpack.ax:=0;
  intr($33,recpack);
  with recpack do
  begin
    mstat:=ax;
    nbuttons:=bx;
  end;
end;

procedure mshow;
begin
  if not MousePresent then exit;
  recpack.ax:=1;
  intr($33,recpack);
end;

procedure mhide;
begin
  if not MousePresent then exit;
  recpack.ax:=2;
  intr($33,recpack);
end;

procedure mpos(var mbt,mx,my:integer);
begin
  if not MousePresent then
  begin
    mbt:=0;mx:=0;my:=0;exit;
  end;
  recpack.ax:=3;
  intr($33,recpack);
  with recpack do
  begin
    mbt:=bx;
    mx:=cx;
    my:=dx;
  end;
end;

procedure mput(mx,my:integer);
begin
  if not MousePresent then exit;
  recpack.ax:=4;
  recpack.cx:=mx;
  recpack.dx:=my;
  intr($33,recpack);
end;

procedure mbutton(num:integer;var button,count,hcur,vcur:integer);
begin
  if not MousePresent then
  begin
    button:=0;count:=0;hcur:=0;vcur:=0;exit;
  end;
  recpack.ax:=num;
  recpack.bx:=button;
  recpack.cx:=0;
  recpack.dx:=0;
  intr($33,recpack);
  button:=recpack.ax;
  count:=recpack.bx;
  hcur:=recpack.cx;
  vcur:=recpack.dx;
end;

procedure mbutpress(var button,count,hcur,vcur:integer);
begin
  mbutton(5,button,count,hcur,vcur);
end;

procedure mbutrelease(var button,count,hcur,vcur:integer);
begin
  mbutton(6,button,count,hcur,vcur);
end;

procedure mhlimit(minpos,maxpos:integer);
begin
  if not MousePresent then exit;
  recpack.ax:=7;
  recpack.cx:=minpos;
  recpack.dx:=maxpos;
  intr($33,recpack);
end;

procedure mvlimit(minpos,maxpos:integer);
begin
  if not MousePresent then exit;
  recpack.ax:=8;
  recpack.cx:=minpos;
  recpack.dx:=maxpos;
  intr($33,recpack);
end;

procedure mgraphcursor(xhot,yhot:integer;var cursor:currarray);
begin
  if not MousePresent then exit;
  with recpack do
  begin
    ax:=9;
    bx:=xhot;
    cx:=yhot;
    dx:=ofs(cursor[0]);
    es:=seg(cursor[0]);
  end;
  intr($33,recpack);
end;

procedure mtextcursor(select,screen,cursor:integer);
begin
  if not MousePresent then exit;
  with recpack do
  begin
    ax:=10;
    bx:=select;
    cx:=screen;
    dx:=cursor;
  end;
  intr($33,recpack);
end;

procedure mmotion(var hcount,vcount:integer);
begin
  if not MousePresent then
  begin
    hcount:=0;vcount:=0;exit;
  end;
  with recpack do
  begin
    ax:=11;
    intr($33,recpack);
    hcount:=cx;
    vcount:=dx;
  end;
end;

procedure mspeed(hor,ver:integer);
begin
  if not MousePresent then exit;
  with recpack do
  begin
    ax:=15;
    cx:=hor;
    dx:=ver;
  end;
  intr($33,recpack);
end;

procedure mwindowhide(x1,y1,x2,y2:integer);
begin
  if not MousePresent then exit;
  with recpack do
  begin
    ax:=16;
    cx:=x1;
    dx:=y1;
    si:=x2;
    di:=y2;
  end;
  intr($33,recpack);
end;

procedure mspeedthreshold(speed:integer);
begin
  if not MousePresent then exit;
  recpack.ax:=19;
  recpack.dx:=speed;
  intr($33,recpack);
end;

begin
  mstatus(i,j);
  if i=0 then MousePresent:=false else MousePresent:=true;
{$IFDEF TESTINIT}
  if MousePresent then writeln('Mouse found and initialised')
    else writeln('Mouse not found.');
  readkey;
{$ENDIF}
end.
