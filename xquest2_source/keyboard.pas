Unit
  Keyboard;

{$UNDEF DEBUG}

{  KEYBOARD UNIT
     Automatically installs new Interrupt 9 handler and removes on
       exit. Check Keys[] or KeyPress[] for information on key
       presses
}


INTERFACE

uses
  Dos;

const
  kSYSREQ=$54;
  kCAPSLOCK=$3A;
  kNUMLOCK=$45;
  kSCROLLLOCK=$46;
  kLEFTCTRL=$1D;
  kLEFTALT=$38;
  kLEFTSHIFT=$2A;
  kRIGHTCTRL=$9D;
  kRIGHTALT=$B8;
  kRIGHTSHIFT=$36;
  kESC=$01;
  kBACKSPACE=$0E;
  kENTER=$1C;
  kSPACE=$39;
  kTAB=$0F;
  kF1=$3B;
  kF2=$3C;
  kF3=$3D;
  kF4=$3E;
  kF5=$3F;
  kF6=$40;
  kF7=$41;
  kF8=$42;
  kF9=$43;
  kF10=$44;
  kF11=$57;
  kF12=$58;
  kA=$1E;
  kB=$30;
  kC=$2E;
  kD=$20;
  kE=$12;
  kF=$21;
  kG=$22;
  kH=$23;
  kJ=$24;
  kK=$25;
  kL=$26;
  kM=$32;
  kN=$31;
  kO=$18;
  kP=$19;
  kQ=$10;
  kR=$13;
  kS=$1F;
  kT=$14;
  kU=$16;
  kV=$2F;
  kW=$11;
  kX=$2D;
  kY=$15;
  kZ=$2C;
  k1=$02;
  k2=$03;
  k3=$04;
  k4=$05;
  k5=$06;
  k6=$07;
  k7=$08;
  k8=$09;
  k9=$0A;
  k0=$0B;
  kMINUS=$0C;
  kEQUAL=$0D;
  kLBRACKET=$1A;
  kRBRACKET=$1B;
  kSEMICOLON=$27;
  kTICK=$28;
  kAPOSTROPHE=$29;
  kBACKSLASH=$2B;
  kCOMMA=$33;
  kPERIOD=$34;
  kSLASH=$35;
  kINS=$D2;
  kDEL=$D3;
  kHOME=$C7;
  kEND=$CF;
  kPGUP=$C9;
  kLARROW=$CB;
  kRARROW=$CD;
  kUARROW=$C8;
  kDARROW=$D0;
  kKEYPAD0=$52;
  kKEYPAD1=$4F;
  kKEYPAD2=$50;
  kKEYPAD3=$51;
  kKEYPAD4=$4B;
  kKEYPAD5=$4C;
  kKEYPAD6=$4D;
  kKEYPAD7=$47;
  kKEYPAD8=$48;
  kKEYPAD9=$49;
  kKEYPADDEL=$53;
  kKEYPADSTAR=$37;
  kKEYPADMINUS=$4A;
  kKEYPADPLUS=$4E;
  kKEYPADENTER=$9C;
  kCNTRLPRTSC=$B7;
  kSHIFTPRTSC=$B7;
  kKEYPADSLASH=$B5;

  SHIFT=$1;
  CTRL=$2;
  ALT=$4;
  CAPSLOCK=$8;

const toascii:array[0..255] of char=
  (  #0, #27, '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', #14, #15,
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p',  #0,  #0, #13,  #0, 'a', 's',
    'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', #0, '''',  #0,  #0, 'z', 'x', 'c', 'v',
    'b', 'n', 'm', ',', '.', '/',  #0,  #0,  #0, ' ',  #0,  #1,  #2,  #3,  #4,  #5,
     #6,  #7,  #8,  #9, #10,  #0,  #0, #24, #25, #26, '-', #21, #22, #23,  #0, #18,
    #19, #20, #16, #17,  #0,  #0,  #0, #11, #12,  #0,  #0,  #0,  #0,  #0,  #0,  #0,
     #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,
     #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,
     #0, #27, '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', #14, #15,
    'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P',  #0,  #0, #13,  #0, 'A', 'S',
    'D', 'F', 'G', 'H', 'J', 'K', 'L', ':',  #0, '"',  #0,  #0, 'Z', 'X', 'C', 'V',
    'B', 'N', 'M', '<', '>', '?',  #0,  #0,  #0, ' ',  #0,  #1,  #2,  #3,  #4,  #5,
     #6,  #7,  #8,  #9, #10,  #0,  #0, #24, #25, #26, '-', #21, #22, #23,  #0, #18,
    #19, #20, #16, #17,  #0,  #0,  #0, #11, #12,  #0,  #0,  #0,  #0,  #0,  #0,  #0,
     #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,
     #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0,  #0);

const
  anykeypressed:boolean=false;
  ShiftState:word=0;
  LastKeyHit:char=#0;
  LastScanCode:byte=0;

var
  Keys: array [0..255] of boolean; { array that holds key values }
  KeyPress: array [0..255] of byte; {array that holds # of times
                                     key pressed since last call}
  e0Flag: byte;

function numkeypresses(key:byte):byte;
procedure clearkeypresses;
function keypressed:boolean;
function yesno(default:char):char;

IMPLEMENTATION
var exitsave: pointer;
    OldInt9: pointer;                { saves location of old OldInt9 vector }

{$F+}

procedure NewInt9; interrupt; assembler;
asm
  cli
  in      al, $60                       { get scan code from keyboard port }
  cmp     al, $E0                       { al = $E0 key ? }
  jne     @@SetScanCode
  mov     [e0Flag], 128
  mov     al, 20h                       { Send 'generic' EOI to PIC }
  out     20h, al
  jmp     @@exit
@@SetScanCode:
  mov     bl, al                        { Save scancode in BL }
  and     bl, 01111111b
  add     bl, [e0Flag]
  xor     bh, bh
  and     al, 10000000b                 { keep break bit, if set }
  xor     al, 10000000b                 { flip bit, 1 means pressed, 0 no }
  rol     al, 1                         { move breakbit to bit 0 }
  cmp     byte ptr [offset keys + bx], 0
  jne     @dontadd                      {ignore typematic keypresses}
  add     [offset KeyPress + bx],al     {add to no. of keypresses}
@dontadd:
  mov     [offset keys + bx], al
  mov     [e0Flag], 0
  or      [anykeypressed],al             {Don't flag release events}
  mov     [lastscancode],bl
  mov     bh,[offset keys + kLEFTSHIFT]
  or      bh,[offset keys + kRIGHTSHIFT]
  ror     bh,1
  add     bl,bh
  xor     bh,bh
  mov     cl,[offset toascii + bx]
  mov     [lastkeyhit],cl
  mov     al, 20h                       { send EOI to PIC }
  out     20h, al
  cmp     bx,kLEFTSHIFT
  je      @@SetShiftState
  cmp     bx,kRIGHTSHIFT
  je      @@SetShiftState
  cmp     bx,kLEFTCTRL
  je      @@SetCtrlState
  cmp     bx,kRIGHTCTRL
  je      @@SetCtrlState
  cmp     bx,kLEFTALT
  je      @@SetAltState
  cmp     bx,kRIGHTALT
  je      @@SetAltState
  cmp     bx,kCAPSLOCK
  je      @@SetCapsLock
  jmp     @@exit

@@SetShiftState:
  cmp     al,1
  je      @@ShiftOn
  and     [ShiftState],(NOT SHIFT)
  jmp     @@exit
@@ShiftOn:
  and     [ShiftState],SHIFT
  jmp     @@exit

@@SetCtrlState:
  cmp     al,1
  je      @@CtrlOn
  and     [ShiftState],(NOT CTRL)
  jmp     @@exit
@@CtrlOn:
  and     [ShiftState],CTRL
  jmp     @@exit

@@SetAltState:
  cmp     al,1
  je      @@AltOn
  and     [ShiftState],(NOT ALT)
  jmp     @@exit
@@AltOn:
  and     [ShiftState],ALT
  jmp     @@exit

@@SetCapsLock:
  cmp     al,1
  jne     @@exit;
  xor     [ShiftState],CAPSLOCK

@@exit:
  sti
end;
{$F-}

function numkeypresses(key:byte):byte;
  {returns # of times key was pressed since last call to this funtion}
begin
  numkeypresses:=keypress[key];
  keypress[key]:=0;
end;

function keypressed:boolean;
begin
  keypressed:=anykeypressed;
  anykeypressed:=false;
end;

procedure clearkeypresses;
begin
  fillchar(keypress,256,0);
  anykeypressed:=false;
end;

function yesno(default:char):char;
var ch:char;
begin
  ch:=#0;
  repeat
    if numkeypresses(kY)>0 then ch:='Y';
    if numkeypresses(kN)>0 then ch:='N';
    if numkeypresses(kESC)>0 then ch:='N';
    if numkeypresses(kENTER)>0 then ch:=default;
  until ch<>#0;
  yesno:=ch;
end;

procedure SetNewInt9;
begin
  GetIntVec($09, OldInt9);         { save old location of INT 09 handler }
  SetIntVec($09, Addr(NewInt9));   { point to new routine }
end;

{$F+}
procedure SetOldInt9;
begin
  exitproc:=exitsave;
  SetIntVec($09, OldInt9);         { point back to original routine }
end;

begin
{$IFNDEF DEBUG}
  SetNewInt9;                      { point to new keyboard routine }
{$ENDIF}
  FillChar(Keys, 256, 0);          { clear the keys array }
  FillChar(KeyPress, 256, 0);      {clear the keypress array}
{$IFNDEF DEBUG}
  exitsave:=exitproc;
  exitproc:=@SetOldInt9;
{$ENDIF}
end.



