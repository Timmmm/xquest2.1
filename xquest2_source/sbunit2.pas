{*****************************************************************************}
{*                                                                           *}
{*                                XQUEST                                     *}
{*                                v 1.3                                      *}
{*                                                                           *}
{*            Copyright (C) 1994 M.Mackey. All rights reserved.              *}
{*                                                                           *}
{*                           SoundBlaster Unit                               *}
{*                                                                           *}
{*****************************************************************************}


unit sbunit2;
{$G+}

{Call SbInit to initialise the sound system, and optionally
 detect the SB port and IRQ values.

 Once SbInit is called, SbPoll MUST BE CALLED at least
 MixSpeed*2/DMABufSize times per second, or the sounds will sound
 "looped". It may be called more often if you don't mind wasting clock
 cycles.

 I recommend that a call to SBPoll be inserted in a timer routine
 to ensure that it is called at the appropriate frequency.

 To add a sound effect use the AddSound procedure (the 'Sound' parameter
 should point to the raw sound data).

 The SbDone procedure MUST be called before exiting the program.

 The MaxSoundEffects variable must not be set to a value larger than the
 compiled CompiledMaximumSoundEffects value. If it is, and the sound
 driver is running, then the results will be undefined and will probably
 lead to a crash.

 Setting the MaxSoundEffects variable to zero will disable sound.

 Vaguely based on the TinyPlay MODplayer code by Carlos Hasan 1993.
}

interface
uses dos;
function SbInit(Detect:boolean):word;
procedure SetSBVolume(Volume:integer);
procedure SbDone;
procedure SbPoll;
function DetectSB:word;
procedure AddSound(var Sound;SoundLength:word);

const
  SbAddr:word=  $220;    {Port address of the SB}
  SbIrq:word=   7;       {IRQ used by the SB}
  SbDMA:byte=   1;       {DMA channel used by the SB}

  MixSpeed:word=11025;   {NOTE: this must be constant for all samples
			  and must be set before initialisation}

  MaxSoundEffects:integer=6;  {May be varied from within the application,
			       but may not exceed the
			       CompiledMaximumSoundEffects constant
			       defined below}

  CompiledMaximumSoundEffects=8;   {absolute maximum no. of sounds.
				     MUST not be exceeded. Determines
				     size of Compander Table, so don't
				     make TOO high}

  MaxSoundVolume=63;
  SoundVolume:integer=48;

  SoundBlasterInitialised:boolean =false;

implementation

const
  DMABufSize= 1536;             {size of DMA buffer, in bytes}
  MixBufSize= DMABufSize div 2; {size of mix buffer, in bytes}
  DefBpm=50; {125}
  SizeOfSoundRec=8;             {size of the Soundrec record}
  SizeOfDMAPortType=6;          {size of the DMAPortType record}

Type Soundrec=record
		sample:pointer;
		length,position:word;
	      end;

     VolTableType=array[0..255] of byte;     {Volume Table}
     CompanderTableType=array[0..255*CompiledMaximumSoundEffects] of byte;

     DMAPorttype=record
		   Page, Address, Length:Word
		 end;
const
  DMAPort:array[1..7] of DMAPortType=          {DMA port information}
     ((Page: $83; Address: $02; Length: $03),
     (Page: $81; Address: $04; Length: $05),
     (Page: $82; Address: $06; Length: $07),
     (Page: $8F; Address: $C0; Length: $C2),
     (Page: $8B; Address: $C4; Length: $C6),
     (Page: $89; Address: $C8; Length: $CA),
     (Page: $8A; Address: $CC; Length: $CE));

{Irq constants}
  IRQInts: array[0..15] of byte =            {interrupt vectors}
    ($08, $09, $0A, $0B, $0C, $0D, $0E, $0F,
     $70, $71, $72, $73, $74, $75, $76, $77);
  IRQcontroller1=    $20;
  IMR1=              $21;    {Interrupt Mask Register}
  IRQcontroller2=    $A0;
  IMR2=              $A1;    {Interrupt Mask Register}
  EOI=$20;                   {to signal End of Interrupt}

var
  DoubleBuffer:array[1..2*DmaBufSize] of byte;
     {sized to ensure that at least DMABufsize bytes are within a physical
      64K chunk: kludgy but it's only a small buffer}

  MixBuffer:array[1..MixBufSize] of byte;

  VolTable:VolTableType;

  DmaFlag,DmaBuffer,BufPtr,BufLen,BpmSamples:word;

  DmaHandler:pointer;

  IRQController,IMR,IRQvector,IRQMask:word;
  SavedIMR1,SavedIMR2:byte;

  Sounds:array[1..CompiledMaximumSoundEffects] of SoundRec;
	 {list of currently playing sounds}

{--------------------------------------------------------------------------
 SbIrqHandler:  Sound Blaster IRQ handler.

 Handles the IRQ and resets the SB for the next DMA transfer loop
--------------------------------------------------------------------------
}
procedure SbIrqHandler;interrupt;assembler;
asm
	mov     dx,[SbAddr]
	add     dx,0Eh       {DATA AVAILABLE port 2xEh}
	in      al,dx        {Acknowledge DSP interrupt}

	sub     dx,02h       {WRITE BUFFER STATUS port 2xCh}

{        SbOut   14h Set 8-bit DMA}
@@Wait:         in      al,dx
		or      al,al
		js      @@Wait
		mov     al,14h
		out     dx,al
{        SbOut   0FFh DMA transfer length LSB (64K)}
@@Wait2:        in      al,dx
		or      al,al
		js      @@Wait2
		mov     al,0FFh
		out     dx,al
{        SbOut   0FFh  DMA transfer length MSB (64K)}
@@Wait3:        in      al,dx
		or      al,al
		js      @@Wait3
		mov     al,0FFh
		out     dx,al

	mov     al,EOI
	mov     dx,IRQController
	out     dx,al      {Send EOI to PIC, acknowledge interrupt}
end;

procedure GetSamples(Buffer:pointer; Count:Word);forward;

{--------------------------------------------------------------------------
 SbPoll:  Sound Blaster Polling

 Calls GetSamples if required. Must be called fairly often.
--------------------------------------------------------------------------
}

procedure SbPoll;assembler;
asm
	pusha
	push    ds
	push    es              {prob'ly not used since Pascal saves
				 regs anyway, but it doesn't really hurt
				 to be safe :). Besides, this may be
				 called from an interrupt handler.}

	mov     ax,SEG @Data
	mov     ds,ax

@@GetDmaCount:

	lea     si,[DMAPort]
	sub     ax,ax
	mov     al,byte ptr [SBDMA]  {DMA number}
	dec     al
	mov     di,SizeOfDMAPortType
	mul     di              {di equals offset into DMAPort array for DMA value used}
	add     si,ax           {[si] points to current DMAPortType array}

	mov     dx,word ptr [si+DMAPortType.Length]
				{dx is the DMA Length port}
	in      al,dx
	mov     cl,al
	in      al,dx
	mov     ch,al           {cx = remaining length of DMA transfer}

	mov     ax,[DmaFlag]    {Switch: are we on the first or second
				 half of the DMA buffer?}
	test    ax,ax
	jne     @@SecondHalf

@@FirstHalf:
	cmp     cx,DmaBufSize/2
	jae     @@Bye            {is the DMA transfer into the second half
				  of the buffer?}
	mov     si,[DmaBuffer]   {Yes, set up first half}
	push    ds
	push    [DmaBuffer]
	push    DmaBufSize/2
	call    GetSamples       {params ptr(DMABuffer), DmaBufSize/2 }
	inc     [DmaFlag]        {Toggle switch}
	jmp     @@Bye

@@SecondHalf:
	cmp     cx,DmaBufSize/2
	jb      @@Bye            {is the DMA transfer into the first half
				  of the buffer?}
	mov     si,[DmaBuffer]
	mov     cx,DmaBufSize/2
	add     si,cx
	push    ds
	push    si
	push    cx
	call    GetSamples       {params ptr(DMABuffer+DmaBufSize/2), DmaBufSize/2 }
	dec     [DmaFlag]        {toggle switch}

@@Bye:

	pop     es
	pop     ds
	popa
end;

procedure TrapIrq2;forward;       {These are for the DetectSB procedure}
procedure TrapIrq3;forward;
procedure TrapIrq5;forward;
procedure TrapIrq7;forward;

{-------------------------------------------------------------------------
DetectSB: detects port and IRQ number of SB device
Returns:        0:      Successful initialisation
		1:      Unable to detect port number
		2:      Unable to detect IRQ number
		3:      Unable to detect DMA number (not implemented)

Sets: SBAddr and SBIrq to detected values.

NOTE: crashes some computers. Known conflict with GUS card, SmartDrive,
      network cards and some other configurations. Will not detect DMA
      number or IRQ's other than 2,3,5,7 (ie no high IRQs). Not really
      recommended (or useful :).
--------------------------------------------------------------------------}

function DetectSb:word;assembler;
asm
	pusha
	push    es

@@ScanPort:
	mov     bx,210h                 { start scanning ports
					  210h, 220h, .. 260h}

@@ResetDSP:
	mov     dx,bx                   { try to reset the DSP.}
	add     dx,06h
	mov     al,1
	out     dx,al
	in      al,dx
	in      al,dx
	in      al,dx
	in      al,dx
	xor     al,al
	out     dx,al
	add     dx,08h
	mov     cx,100
@@WaitID:
	in      al,dx
	or      al,al
	js      @@GetID
	dec     cx
	jnz     @@WaitID
	jmp     @@NextPort
@@GetID:
	sub     dx,04h
	in      al,dx
	cmp     al,0AAh
	je      @@Found
	add     dx,04h
	dec     cx
	jnz     @@WaitID

@@NextPort:
	add     bx,10h                  {if not response,}
	cmp     bx,260h                 { try the next port.}
	jbe     @@ResetDSP
	jmp     @@PortFail

@@Found:
	mov     [SbAddr],bx             { SB Port Address Found!}

@@ScanIRQ:
	cli

	in      al,IMR1                 { save the IMR.}
	mov     bl,al

	mov     al,11111111b            { disable all the IRQs.}
	out     21h,al

	xor     ax,ax                   { trap the IRQs 2,3,5,7.}
	mov     es,ax

@@SaveIrqs:
	mov     ax,es:[28h]             { irq2}
	mov     dx,es:[2Ah]
	push    ax
	push    dx

	mov     ax,es:[2Ch]             { irq3}
	mov     dx,es:[2Eh]
	push    ax
	push    dx

	mov     ax,es:[34h]             { irq5}
	mov     dx,es:[36h]
	push    ax
	push    dx

	mov     ax,es:[3Ch]             { irq7}
	mov     dx,es:[3Eh]
	push    ax
	push    dx

@@SetIrqs:
	mov     ax,offset TrapIrq2      { irq2}
	mov     es:[28h],ax
	mov     es:[2Ah],cs

	mov     ax,offset TrapIrq3      { irq3}
	mov     es:[2Ch],ax
	mov     es:[2Eh],cs

	mov     ax,offset TrapIrq5      { irq5}
	mov     es:[34h],ax
	mov     es:[36h],cs

	mov     ax,offset TrapIrq7      { irq7}
	mov     es:[3Ch],ax
	mov     es:[3Eh],cs

@@EnableIrqs:
	mov     al,bl                   { enable IRQs 2,3,5,7.}
	and     al,01010011b

	out     IMR1,al

	sti

	mov     [SbIrq],0               { clear the IRQ level.}

	mov     dx,[SbAddr]             { tells to the SB to}
	add     dx,0Ch                  { generate an IRQ!}
@@WaitSb:
	in      al,dx
	or      al,al
	js      @@WaitSb
	mov     al,0F2h
	out     dx,al

	xor     cx,cx                   { wait until IRQ level}
@@WaitIRQ:
	cmp     [SbIrq],0               { is changed or timeout.}
	jne     @@IrqOk
	dec     cx
	jne     @@WaitIRQ

@@IrqOk:
	mov     al,bl                   { restore IMR.}
	out     IMR1,al

@@RestoreIrqs:
	cli                             { restore IRQ vectors.}

	xor     ax,ax
	mov     es,ax

	pop     dx                      { irq7}
	pop     ax
	mov     es:[3Ch],ax
	mov     es:[3Eh],dx

	pop     dx                      { irq5}
	pop     ax
	mov     es:[34h],ax
	mov     es:[36h],dx

	pop     dx                      { irq3}
	pop     ax
	mov     es:[2Ch],ax
	mov     es:[2Eh],dx

	pop     dx                      { irq2}
	pop     ax
	mov     es:[28h],ax
	mov     es:[2Ah],dx

	cli

	cmp     [SbIrq],0               { IRQ level was changed?}
	je      @@IRQFail               { no, fail.}

@@Success:
	pop     es
	popa                            { Return to caller.}
	mov     ax,0                    { Success}
	ret

@@PortFail:
	pop     es
	popa
	mov     ax,1                    { Failed to detect port}
	ret

@@IRQFail:
	pop     es
	popa
	mov     ax,2                    {Failed to detect IRQ}
	ret
end;


procedure TrapIrq;assembler;
asm
		push    dx                      { General IRQ trapper }
		push    ds                      { used for IRQ autodetect.}
		mov     dx,SEG @Data
		mov     ds,dx
		mov     [SbIrq],ax              { save IRQ level.}
		mov     dx,[SbAddr]
		add     dx,0Eh
		in      al,dx                   { SB acknowledge.}
		mov     al,20h
		out     20h,al                  { Hardware acknowledge.}
		pop     ds
		pop     dx
		pop     ax
		iret          {can't use _interrupt_ directive here,}
			      {compiler barfs on jmp commands in TrapIRQn procs}
end;

procedure TrapIrq2;assembler;
asm
		push    ax
		mov     ax,2
		jmp     TrapIrq
end;

procedure TrapIrq3;assembler;
asm
		push    ax
		mov     ax,3
		jmp     TrapIrq
end;

procedure TrapIrq5;assembler;
asm
		push    ax
		mov     ax,5
		jmp     TrapIrq
end;

procedure TrapIrq7;assembler;
asm
		push    ax
		mov     ax,7
		jmp     TrapIrq
end;



{--------------------------------------------------------------------------
 MixSound:  Mixes one sound into the buffer.
	    Checks for over/underflow

IMPROVEMENTS: Should mix all sounds into 16-bit buffer, then
	      reduce buffer to 8-bit after mixing. The current
	      procedure gives mixing errors.


  In:
   ds:si -  Pointer to SoundRec.
   ds:di -  pointer to MixBuffer.
    cx   -  Buffer Size.
  Preserves: si,ax,bp
--------------------------------------------------------------------------
}
procedure MixSound;assembler;
asm
		push    bp       {TP requires bp to be saved}
		les     dx,[si+Soundrec.Sample]
		mov     bx,[si+Soundrec.Position]
		mov     bp,[si+Soundrec.Length]
		cmp     bx,bp                      {is sample empty?}
		jae     @@EmptySample
		push    si
		add     bx,dx                      
		add     bp,dx                      {es:bp -> end of sample}
		mov     si,bx                      {es:si -> next sample byte}
		push    ax
		push    dx
		xor     ax,ax
		mov     bx,ax
		mov     dx,ax

@@MixSamp:
		mov     al,[di]                    {get byte from Mixbuffer}
		mov     bl,es:[si]                 {get sample value}
		mov     dl,byte ptr [VolTable+bx]  {and convert via VolumeTable}
		add     dx,ax                      {and add the two together}
		sub     dx,80h                     {and adjust the zero}
		jns     @@OKNeg
		mov     dx,0                       {underflow: set to zero}
@@OKNeg:        cmp     dh,0
		jz      @@OKPos
		mov     dx,0ffh                    {overflow: set to 0FFh}
@@OKPos:        mov     [di],dl                    {place new value in MixBuffer}
		inc     di
		inc     si
		cmp     si,bp                      {end of sample?}
		jae     @@MixBye
		dec     cx                         {end of buffer?}
		jnz     @@MixSamp

@@MixBye:       mov     bx,si
		pop     dx
		pop     ax
		pop     si
		sub     bx,dx                      {number of bytes in sample left to be mixed}
		mov     [si+SoundRec.Position],bx
@@EmptySample:
		pop     bp
end;



{--------------------------------------------------------------------------
 GetSamples:  Returns the next chunk of samples to be played.
  In:
    Buffer  - Address of current half of DMA Buffer.
    Count   - Half-size of DMA Buffer (mix one half while playing the other)

  Calls MixSound for each sound in queue.
--------------------------------------------------------------------------
}

procedure GetSamples(Buffer:pointer; Count:Word);assembler;
asm
		push    ds
		cld

		les     di,[Buffer]      {es:di points to DMA buffer}
		mov     bx,[Count]

@@NextChunk:    cmp     [BufLen],0
		jne     @@CopyChunk

		push    bx
		push    di
		push    es

@@MixChunk:     lea     di,[MixBuffer]
		mov     cx,[BpmSamples]
		mov     [BufPtr],di
		mov     [BufLen],cx

		mov     ax,ds
		mov     es,ax
		mov     al,80h
		rep     stosb

		mov     ax,MaxSoundEffects
		cmp     ax,0
		jle     @@MixFinished   {allow for zero MaxSoundEffects}
		mov     si,Offset Sounds
@@MixLoop:
		mov     di,[BufPtr]
		mov     cx,[BufLen]
		call    MixSound
		add     si,SizeOfSoundRec
		dec     ax
		jnz     @@MixLoop

@@MixFinished:
		pop     es
		pop     di
		pop     bx

@@CopyChunk:    mov     cx,[BufLen]
		cmp     cx,bx              {bx = Count}
		jbe     @@MoveChunk        {if cx<bx }
		mov     cx,bx              {make cx least of cx, bx}

@@MoveChunk:    mov     si,[BufPtr]
		add     [BufPtr],cx
		sub     [BufLen],cx
		sub     bx,cx
		rep     movsb
		test    bx,bx
		jnz     @@NextChunk

		pop     ds
end;

{--------------------------------------------------------------------------
; SetSBVolume: Sets up volume table  (max. Volume 63)
;    Destroys  cx,ax,bx; preserves others
;--------------------------------------------------------------------------
}

procedure SetSBVolume(Volume:integer);assembler;
asm
		mov     cx,256
		mov     bx,0
		mov     dx,0
@@VolLoop:      mov     ax,word ptr [Volume]
		cmp     ax,MaxSoundVolume
		jl      @@InBounds
		mov     ax,MaxSoundVolume
@@InBounds:
		sub     dx,127
		mul     dx
		sar     ax,6
		add     ax,127
		mov     byte ptr VolTable[bx],al
		inc     bx
		mov     dx,bx
		loop    @@VolLoop
end;


{--------------------------------------------------------------------------
; StartPlaying: Initializes the Sound System.
		Clears buffers, sets up volume table at max volume.
;--------------------------------------------------------------------------
}

procedure StartPlaying;assembler;
asm
		pusha
		push    ds
		push    es

		mov     ax,[MixSpeed]
		xor     dx,dx
		mov     bx,24*DefBpm/60
		div     bx
		mov     [BpmSamples],ax

@@ClearSounds:  mov     di,offset Sounds
		mov     ax,ds
		mov     es,ax
		mov     cx,8*CompiledMaximumSoundEffects
		xor     ax,ax
		cld
		rep     stosb

		mov     [BufPtr],ax
		mov     [BufLen],ax

		push    word ptr SoundVolume
		call    SetSBVolume

		pop     es
		pop     ds
		popa
end;

{--------------------------------------------------------------------------
 SbInit: Initializes the Sound Blaster Driver.

 In:       DETECT:      Calls DetectSB if true
 Returns:       0:      Successful initialisation
		1:      Unable to detect port number  (only if DETECT)
		2:      Unable to detect IRQ number   (only if DETECT)
		3:      Unable to detect DMA no.      (not implemented)
		4:      Unable to reset DSP
--------------------------------------------------------------------------
}
function SbInit(Detect:boolean):word;assembler;
var StopMask:byte;
asm
	push    ds

	mov     ax,CompiledMaximumSoundEffects
	cmp     ax,[MaxSoundEffects]
	jge     @@OK
	mov     [MaxSoundEffects],ax
@@OK:
	cmp     [Detect],0
	jz      @@DontDetect

	call    DetectSB      {detect port and IRQ}
	test    ax,ax
	jz      @@DontDetect

	ret                   {return result from DetectSB directly to user}

@@DontDetect:
	call    StartPlaying  {set up and clear DMA buffer and mixbuffer}

@@ResetDsp:
	mov     dx,[SbAddr]
	add     dx,06h       {reset port 2x6h}
	mov     al,1
	out     dx,al
	in      al,dx
	in      al,dx
	in      al,dx
	in      al,dx
	xor     al,al
	out     dx,al
	mov     cx,100h
@@WaitId:
	mov     dx,[SbAddr]
	add     dx,0Eh       {data available port 2xEh}
	in      al,dx
	or      al,al
	js      @@GetID
	dec     cx
	jnz     @@WaitId
	jmp     @@SBNotDetected  {Reset failed}
@@GetId:
	mov     dx,[SbAddr]
	add     dx,0Ah           {READ DATA port 2xAh}
	in      al,dx
	cmp     al,0AAh          {DATA READY value AAh}
	je      @@SbOk
	dec     cx
	jnz     @@WaitId
	jmp     @@SBNotDetected

@@SbOk:

	cmp     SBIrq,7
	jg      @@HighIRQ
	mov     [IRQController],IRQController1
	mov     [IMR],IMR1
	jmp     @@IRQVarsSet
@@HighIRQ:
	mov     [IRQController],IRQController2
	mov     [IMR],IMR2
@@IRQVarsSet:

	mov     cx,SBIrq
	and     cx,$07     {modulo 8}
	mov     ax,1
	shl     ax,cl
	mov     [StopMask],al

	in      al,IMR1
	mov     [SavedIMR1],al

	in      al,IMR2
	mov     [SavedIMR2],al

@@SetBuffer:
	mov     [DmaFlag],0
	mov     ax,offset DoubleBuffer
	mov     [DmaBuffer],ax
	mov     dx,ds
	mov     bx,dx
	shr     dh,4
	shl     bx,4
	add     ax,bx
	adc     dh,0
	mov     cx,ax
	neg     cx
	cmp     cx,DmaBufSize
	jae     @@SetDma
	add     [DmaBuffer],cx    {This stuff ensures that the
		      DMA buffer is OK ie doesn't cross a 64K boundary}
	add     ax,cx
	adc     dh,0

@@SetDma:
	mov     bx,ax
	mov     cx,DmaBufSize
	dec     cx

	cmp     [SBDMA],3
	jg      @@DoDMA04

	mov     al,04h
	add     al,[SbDMA]
	out     0Ah,al

	xor     al,al
	out     0Ch,al

	lea     si,[DMAPort]
	sub     ax,ax
	mov     al,[SBDMA]
	dec     al
	mov     di,SizeOfDMAPortType
	push    dx
	mul     di
	add     si,ax           {[si] points to current DMAPortType array}

	mov     dx,word ptr [si+DMAPortType.Address]
	mov     al,bl
	out     dx,al
	mov     al,bh
	out     dx,al           {Set address}

	pop     ax
	mov     dx,word ptr [si+DMAPortType.Page]
	mov     al,ah
	out     dx,al

	mov     dx,word ptr [si+DMAPortType.Length]
	mov     al,cl
	out     dx,al
	mov     al,ch
	out     dx,al

	mov     ax,58h
	add     al,[SbDMA]      {Set block mode, correct channel}
	out     0Bh,al

	mov     al,[SbDMA]
	out     0Ah,al
	jmp     @@ClearBuffer

@@DoDMA04:
	mov     al,[SbDMA]
	add     al,04h
	out     0D4h,al

	xor     al,al
	out     0D8h,al

	lea     si,[DMAPort]
	sub     ax,ax
	mov     al,[SBDMA]
	dec     al
	mov     di,SizeOfDMAPortType
	push    dx
	mul     di
	add     si,ax           {[si] points to current DMAPortType array}

	mov     dx,word ptr [si+DMAPortType.Address]
	mov     al,bl
	out     dx,al
	mov     al,bh
	out     dx,al           {Set address}

	pop     ax
	mov     dx,word ptr [si+DMAPortType.Page]
	mov     al,ah
	out     dx,al

	mov     dx,word ptr [si+DMAPortType.Length]
	mov     al,cl
	out     dx,al
	mov     al,ch
	out     dx,al

	mov     al,[SBDMA]      {set block mode, correct channel}
	sub     al,4
	add     al,58h          {ineff. but more clear}
	out     0D6h,al

	mov     al,[SbDMA]
	and     al,00000011b
	out     0D4h,al

@@ClearBuffer:
	mov     di,[DmaBuffer]
	mov     cx,DmaBufSize
	mov     ax,ds
	mov     es,ax
	mov     al,7fh           {all 127's therefore silent}
	cld
	rep     stosb

@@SetIrq:
	cli                     { Clear interrupt flag}

	mov     dx,[IMR]
	in      al,dx
	or      al,[StopMask]
	out     dx,al           {turn off SB IRQ}

	xor     ax,ax
	mov     es,ax

	lea     si,[IRQInts]
	mov     bx,[SbIrq]               { hi(SBIrq)=0}
	mov     bl,byte ptr [si+bx]      { IRQInts[SbIRQ] }
	shl     bx,2

	mov     ax,es:[bx]
	mov     dx,es:[bx+2]
	mov     Word Ptr [DmaHandler],ax
	mov     Word Ptr [DmaHandler+2],dx

	mov     ax,offset SbIrqHandler
	mov     es:[bx],ax
	mov     es:[bx+2],cs    {sets IRQ handler & saved old handler}

	mov     dx,[IMR]
	in      al,dx
	mov     bl,[StopMask]
	not     bl
	and     al,bl
	out     dx,al           {turn on SB IRQ}

	sti                     {Set interrupt flag}


	mov     dx,[SbAddr]
	add     dx,0Ch
{        SbOut   0D1h  Turn Speaker On}
@@Wait:         in      al,dx
		or      al,al
		js      @@Wait
		mov     al,0D1h
		out     dx,al
	mov     ax,1000
	mul     ax
	div     [MixSpeed]
	neg     al
	mov     ah,al
	mov     dx,[SbAddr]
	add     dx,0Ch
{        SbOut   40h Set DMA transfer freq.}
@@Wait2:        in      al,dx
		or      al,al
		js      @@Wait2
		mov     al,40h
		out     dx,al
{        SbOut   ah Output TIME_CONSTANT=256-1000000/frequency}
@@Wait3:        in      al,dx
		or      al,al
		js      @@Wait3
		mov     al,ah
		out     dx,al
@@StartDma:
{        SbOut   14h Set 8-bit DMA}
@@Wait4:        in      al,dx
		or      al,al
		js      @@Wait4
		mov     al,14h
		out     dx,al
{        SbOut   0FFh Data length LSB (64K)}
@@Wait5:        in      al,dx
		or      al,al
		js      @@Wait5
		mov     al,0FFh
		out     dx,al
{        Sbout   0FFh Data length MSB, set for 64K}
@@Wait6:        in      al,dx
		or      al,al
		js      @@Wait6
		mov     al,0FFh
		out     dx,al

@@Exit:

	pop     ds
	mov     ax,0            {return success}
	mov     [SoundBlasterInitialised],1
	jmp     @@End

@@SBNotDetected:

	pop     ds
	mov     [SoundBlasterInitialised],0
	mov     ax,4            {return DSP reset failed}
@@End:
end;



{--------------------------------------------------------------------------
; SbDone:  Shut Down the Sound Blaster Driver.
;--------------------------------------------------------------------------
}
procedure SbDone;assembler;
asm
	mov     word ptr [SoundBlasterInitialised],FALSE
	xor     ax,ax
	mov     es,ax

	lea     si,[IRQInts]
	mov     bx,[SbIrq]               { hi(SBIrq)=0}
	mov     bl,byte ptr [si+bx]      { IRQInts[SbIRQ] }
	shl     bx,2                     { convert to interrupt vector table offset}

	cli

	mov     ax,Word Ptr [DmaHandler]
	mov     dx,Word ptr [DmaHandler+2]
	mov     es:[bx],ax
	mov     es:[bx+2],dx             {restore original INT handler}

	mov     al,[SavedIMR1]
	out     IMR1,al
	mov     al,[SavedIMR2]
	out     IMR2,al
	sti                             {restore original IMRs}

	cmp     [SBDMA],3
	jg      @@DoDMA04


	mov     al,[SbDMA]
	out     0Ah,al                  {Set proper DMA mask}

	xor     al,al
	out     0Ch,al                  {Reset DMA}
	jmp     @@DMADone
@@DoDMA04:
	mov     al,[SbDMA]
	out     0D4h,al                 {Set proper DMA mask}

	xor     al,al
	out     0D8h,al                 {Reset DMA}

@@DMADone:
	mov     dx,[SbAddr]
	add     dx,0Ch

{        SbOut   0D0h      DMA stop}
@@Wait:         in      al,dx
		or      al,al
		js      @@Wait
		mov     al,0D0h
		out     dx,al
{        SbOut   0D3h      Speaker off}
@@Wait2:        in      al,dx
		or      al,al
		js      @@Wait2
		mov     al,0D3h
		out     dx,al

end;


{
procedure AddSound(var Sound;SoundLength:word);  (version 1)
begin
  inc(SoundNum);
  if SoundNum>MaxSoundEffects then SoundNum:=1;
  with Sounds[SoundNum] do
  begin
    sample:=@Sound;
    position:=0;
    length:=SoundLength;
  end;
end;
}

{--------------------------------------------------------------------------
; AddSound:  Adds a new sound to the sound queue, overwriting the sound
;            with the least playing time left.
;--------------------------------------------------------------------------
}

procedure AddSound(var Sound;SoundLength:word);  {version 2}
{Stuffed: should always scan all sounds or wrap around to do so...???}
var i:integer;
begin
  asm
    cmp  [MaxSoundEffects],0
    jle  @@Finished
    xor  cx,cx
    mov  bx,cx
    mov  di,cx
    mov  si,07FFFh                {MAXINT}
@@Scanloop:
    mov  dx,word ptr [Sounds+Soundrec.length+bx]
    mov  ax,word ptr [Sounds+Soundrec.position+bx]
    sub  dx,ax                    {dx = # samples remaining}
    cmp  dx,si
    jge  @@Smaller
    mov  si,dx
    mov  di,cx                    {store this sound number}
@@Smaller:
    add  bx,SizeOfSoundRec        {next SoundRec}
    inc  cx
    cmp  cx,[MaxSoundEffects]
    jle  @@Scanloop
       {di now holds the sound# with least time left}

    mov  ax,di
    mov  bx,SizeOfSoundRec
    mul  bx
    mov  bx,ax     {bx holds offset for Sounds array}
    mov  ax,word ptr [Sound]
    mov  word ptr [Sounds+SoundRec.sample+bx],ax
    mov  ax,word ptr [Sound+2]
    mov  word ptr [Sounds+SoundRec.sample+bx+2],ax  {Set pointer to sound}
    mov  word ptr [Sounds+SoundRec.position+bx],0
    mov  ax,word ptr [SoundLength]
    mov  word ptr [Sounds+SoundRec.length+bx],ax    {Set Soundlength}
@@Finished:
  end;
end;


end.
