{**************************************************************************}
{*                                                                        *}
{*                         JOYSTICK.PAS                                   *}
{*                                                                        *}
{*            A sample joystick unit working off PIT 2                    *}
{*                                                                        *}
{*                 Copyright (C) 1995 Mark Mackey.                        *}
{*                                                                        *}
{*  Permission to freely use, copy, or modify this software or its        *}
{*  documentation for any purpose is hereby granted provided that the     *}
{*  original authors are acknowledged.                                    *}
{*                                                                        *}
{**************************************************************************}

{ Based on JOY.PAS by Mark Feldman (u914097@student.canberra.edu.au)
  and on TIMERX by Peter Sprenger: (Pete@amber.dinoco.de)

/*TIMERX
 Copyright 1993 by Peter Sprenger   Pete@amber.dinoco.de
 *                   5014 Kerpen 3
 *                   Germany
*/

}


unit JoyStick;
Interface
uses crt;
{ Define constants for use as JoystickButton and JoystickPosition parameters }
const JoystickAButton1 = $10;
      JoystickAButton2 = $20;
      JoystickBButton1 = $40;
      JoystickBButton2 = $80;
      JoystickAAxisX   = $01;
      JoystickAAxisY   = $02;
      JoystickBAxisX   = $04;
      JoystickBAxisY   = $08;

const JoySelect:byte=JoyStickAAxisX and JoyStickAAxisY
                      and JoyStickBAxisX and JoyStickBAxisY;

                      {selects which joystick channels to monitor}
      JoyWidth:word=255;      {Width of scaled joystick response
                               (ie sensitivity)                }

      JoyAPresent:boolean=false;
      JoyBPresent:boolean=false;
      JoyAButton1Down:boolean=false;
      JoyAButton2Down:boolean=false;
      JoyBButton1Down:boolean=false;
      JoyBButton2Down:boolean=false;
      JoyStickACalibrated:boolean=false;
      JoyStickBCalibrated:boolean=false;

type JoyCalibrationType=record
                          XMin,XCentreMin,XCentremax,XMax,
                          YMin,YCentreMin,YCentremax,YMax:word;
                        end;
         {calibration values. Must be set by host program:
           see sample code below}
var JoyCal:array[1..2] of JoyCalibrationType;
    tax,tay,tbx,tby:word;  {raw potentiometer values}

function JoystickButtonDown:byte;
function JoystickButtonPressed: byte;
procedure ReadJoyStick;
procedure JoyPos(JoyNum:byte;var XPos,Ypos:integer);
procedure JoyPos2(JoyNum:byte;var XPos,Ypos:integer);
procedure InitJoyUnit;
procedure ExampleCalibrateJoyStick;

Implementation

const JOYSTICKPORT = $201;
      PITCONTROL   = $43;
      PIT2         = $42;

procedure InitTimer2;assembler;
{Initialise Timer 2}
asm
  in al,$61   { no signal on speaker! }
  and al,$fd
  or  al,1
  out $61,al

  mov al,$b4  { program timer 2 with modus 3 }
  out PITControl,al  { and counter value of 0 (2^16)}
  mov al,0
  out PIT2,al
  out PIT2,al
end;

function JoystickButtonDown:byte;assembler;
{Returns ANDed JoyStickXButtonY values}
asm
  xor ax,ax
  mov dx,JoyStickPort
  in  al,dx
  not al
  and al,(JoyStickAButton1 or JoyStickAButton2 or JoyStickBButton1 or JoyStickBButton2)
end;

function JoystickButtonPressed: byte;
{returns and'ed button numbers of buttons which have been pressed since
 the last call to this function. Returns zero if no buttons have been
 pressed}
var down:boolean;
    button,ButtonsCurrentlyDown:byte;
begin
  button:=0;
  ButtonsCurrentlyDown:=JoystickButtonDown;
  down:=(ButtonsCurrentlyDown and JoyStickAButton1)<>0;
  if down and not JoyAButton1Down then button:=button or JoyStickAButton1;
  JoyAButton1Down:=down;
  down:=(ButtonsCurrentlyDown and JoyStickAButton2)<>0;
  if down and not JoyAButton2Down then button:=button or JoyStickAButton2;
  JoyAButton2Down:=down;
  down:=(ButtonsCurrentlyDown and JoyStickAButton1)<>0;
  if down and not JoyBButton1Down then button:=button or JoyStickAButton1;
  JoyBButton1Down:=down;
  down:=(ButtonsCurrentlyDown and JoyStickAButton2)<>0;
  if down and not JoyBButton2Down then button:=button or JoyStickAButton2;
  JoyBButton2Down:=down;
  JoyStickbuttonPressed:=button;
end;

procedure ReadJoyStick;assembler;
{ ReadTJoy reads the potentiometer values from the joysticks and
  sets the variables tax,tay,tbx,tby equal to the number of CLK's
  (1 CLK equals approx. 824 ns) since the joystick was polled.

  The JoySelect variable determines which channels to query. This prevents
  from hitting the timeout, if one of the joysticks are missing.
  Set JoySelect to the channels of interest. When ReaadJoystick
  comes back the channels that are missing have a zero in their
  variables. (NB: JoySelect is set on initialisation and the
  JoyAPresent and JoyBPresent variables set accordingly)

}

asm
{  call InitTimer2;
  If you are using timer 2 for any other purpose in your code at any
  time, or if the code behaves erratically, then uncomment this line.
  Normally Timer 2 is set up in the initialisation code and remains
  untouched.}

  mov tax,0;  { initialise values }
  mov tay,0;
  mov tbx,0;
  mov tby,0;

  mov dx,JoyStickPort;
  out dx,al;

  mov al,$80;  { latch timer 2 }
  out PITControl,al;  { save value in bx }
  in al,PIT2;
  mov bl,al;
  in al,PIT2;
  mov bh,al;

@joyloop:
  in al,dx;     { read joyport bits to al }

  test al,JoyStickAAxisX;  { timer bit set? }
  jnz @notax;
  test tax,$ffff; { is tax non-zero? If so then value already obtained.}
  jz @getax;
@notax:
  test al,JoyStickAAxisY;
  jnz @notay;
  test tay,$ffff;
  jz @getay;
@notay:
  test al,JoyStickBAxisX;
  jnz @notbx;
  test tbx,$ffff;
  jz @getbx;
@notbx:
  test al,JoyStickBAxisY;
  jnz @notby;
  test tby,$ffff;
  jz @getby;
@notby:
  test al,JoySelect;  {Have all the bits settled?}
  jz @end;            {Yes, finished}

  mov al,$80;
  out PITControl,al;
  in al,PIT2;
  mov ah,al;
  in al,PIT2;
  xchg ah,al;     {latch timer 2 value into ax}
  sub ax,bx;
  neg ax;         {ax holds number of CLK's since timing started}
  cmp ax,2000;    {abort routine after 5000 CLK (5 ms)}
  jg @end;
  jmp @joyloop;

@getax:
  mov al,$80;
  out PITControl,al;
  in al,PIT2;
  mov ah,al;
  in al,PIT2;
  xchg ah,al;
  sub ax,bx;
  neg ax;         {ax holds number of CLK's since timing started}
  mov tax,ax;
  jmp @joyloop;

@getay:
  mov al,$80;
  out PITControl,al;
  in al,PIT2;
  mov ah,al;
  in al,PIT2;
  xchg ah,al;
  sub ax,bx;
  neg ax;         {ax holds number of CLK's since timing started}
  mov tay,ax;
  jmp @joyloop;

@getbx:
  mov al,$80;
  out PITControl,al;
  in al,PIT2;
  mov ah,al;
  in al,PIT2;
  xchg ah,al;
  sub ax,bx;
  neg ax;         {ax holds number of CLK's since timing started}
  mov tbx,ax;
  jmp @joyloop;

@getby:
  mov al,$80;
  out PITControl,al;
  in al,PIT2;
  mov ah,al;
  in al,PIT2;
  xchg ah,al;
  sub ax,bx;
  neg ax;         {ax holds number of CLK's since timing started}
  mov tby,ax;
  jmp @joyloop;
@end:

end;

procedure JoyPos(JoyNum:byte;var XPos,Ypos:integer);
{returns the position of the specified joystick using the correct
 calibration values and adjusted to give a value between +-JoyWidth.
 This procedure uses fairly slow floating-point arithmetic, but given
 the time taken to call ReadJoyStick it's not really worthwhile
 converting this stuff to fixed point.

 JoyNum indicates which joystick we are looking at (1 or 2), and
  the X and Y positions are returned in XPos and YPos
}
var JoyX,JoyY:word;
begin
  ReadJoyStick;
  if JoyNum=1 then
  begin
    JoyX:=tax;JoyY:=tay;
  end else
  begin
    JoyX:=tbx;JoyY:=tby;
  end;
  XPos:=0;YPos:=0;
  with JoyCal[JoyNum] do
  begin
    if JoyX<XCentreMin then
    begin
      XPos:=round(-(XCentreMin-JoyX)/(XCentreMin-XMin)*(JoyWidth));
      if XPos<-JoyWidth then XPos:=-JoyWidth;
    end else
    if JoyX>XCentreMax then
    begin
      XPos:=round((JoyX-XCentreMax)/(XMax-XCentreMax)*(JoyWidth));
      if Xpos>JoyWidth then XPos:=JoyWidth;
    end;
    if JoyY<YCentreMin then
    begin
      YPos:=round(-(YCentreMin-JoyY)/(YCentreMin-YMin)*(JoyWidth));
      if YPos<-JoyWidth then YPos:=-JoyWidth;
    end else
    if JoyY>YCentreMax then
    begin
      YPos:=round((JoyY-YCentreMax)/(YMax-YCentreMax)*(JoyWidth));
      if YPos>JoyWidth then YPos:=JoyWidth;
    end;
  end;
end;

procedure JoyPos2(JoyNum:byte;var XPos,Ypos:integer);
{returns the position of the specified joystick using the correct
 calibration values and adjusted to give a value between +-JoyWidth.
 This procedure does NOT use the 10% 'dead' area in the centre of the
 joystick: this gives better control for small displacements}

var JoyX,JoyY,XCentre,YCentre:word;
begin
  ReadJoyStick;
  if JoyNum=1 then
  begin
    JoyX:=tax;JoyY:=tay;
  end else
  begin
    JoyX:=tbx;JoyY:=tby;
  end;
  XPos:=0;YPos:=0;
  with JoyCal[JoyNum] do
  begin
    XCentre:=(XCentreMin+XCentreMax div 2);
    YCentre:=(YCentreMin+YCentreMax div 2);
    if JoyX<XCentreMin then
    begin
      XPos:=round(-(XCentre-JoyX)/(XCentre-XMin)*(JoyWidth));
      if XPos<-JoyWidth then XPos:=-JoyWidth;
    end else
    if JoyX>XCentreMax then
    begin
      XPos:=round((JoyX-XCentre)/(XMax-XCentre)*(JoyWidth));
      if Xpos>JoyWidth then XPos:=JoyWidth;
    end;
    if JoyY<YCentreMin then
    begin
      YPos:=round(-(YCentre-JoyY)/(YCentre-YMin)*(JoyWidth));
      if YPos<-JoyWidth then YPos:=-JoyWidth;
    end else
    if JoyY>YCentreMax then
    begin
      YPos:=round((JoyY-YCentre)/(YMax-YCentre)*(JoyWidth));
      if YPos>JoyWidth then YPos:=JoyWidth;
    end;
  end;
end;

procedure InitJoyUnit;
begin
  InitTimer2;
  JoySelect:=$f;
  ReadJoyStick;
  JoyAPresent:=(tax or tay)<>0;
  JoyBPresent:=(tbx or tby)<>0;
  if not JoyAPresent then JoySelect:=JoySelect xor $3;
  if not JoyBPresent then JoySelect:=JoySelect xor $C;
end;

procedure ExampleCalibrateJoyStick;
var ch:char;

  procedure PositionOnButtonPress{(min:boolean)};
  {Calculates a running average of joystick positions until a button
   is pressed. This gives more consistent results than a single read.
  }
  var rtax,rtay:real;
  begin
    ReadJoyStick;
    rtax:=tax;rtay:=tay;
    repeat
      ReadJoyStick;
      rtax:=(rtax*8+tax)/9;
      rtay:=(rtay*8+tay)/9;
      if keypressed then ch:=readkey;
    until (JoyStickButtonPressed<>0) or (ch=#27);
    tax:=round(rtax);
    tay:=round(rtay);
  end;

begin
  if not JoyAPresent then
  begin
    writeln('Joystick not found');
    exit;
  end;
  with JoyCal[1] do
  begin
    writeln('Centre stick and press button');
    PositionOnButtonPress;
    if ch<>#27 then
    begin
      XCentreMin:=round(tax*0.98);
      YCentreMin:=round(tay*0.98);
      XCentreMax:=round(tax*1.02);
      YCentreMax:=round(tay*1.02);
      writeln('Move to top left and press button');
      PositionOnButtonPress;
      if ch<>#27 then
      begin
        XMin:=tax;YMin:=tay;
        writeln('Move to bottom right and press button');
        PositionOnButtonPress;
        JoyStickACalibrated:=true;
      end;
    end;
    XMax:=tax;YMax:=tay;
  end;
end;

begin
  InitJoyUnit;      {Remove this is you want to manually initialise
                     the joystick routines from your own program}
end.


