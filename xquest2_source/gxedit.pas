program graphx;
uses crt,graph,mouse,filexist;
const xsize:integer=13;
      bmxsize:integer=16;
      ysize:integer=13;
      maximages:integer=3;
      background=0;
      filename='\xq_gfx\enemy10';
      filename2='\xq_gfx\enemy10';

var color,number,i,j,k,l,x,y,oldx,oldy,graphdriver,graphmode,symmetry:integer;
    p,out:pointer;
    an:array[0..100] of pointer;
    f:file;
    ch:char;
    temp:byte;

{$I palette.inc}

procedure SVgaDriver; external;
{$L sVga256.obj}

procedure putbit(x,y,color:integer);
begin
  putpixel(x div 10+450+40*(number div 8),y div 10+20+25*(number and 7),color);
  setfillstyle(solidfill,color);
  bar(x,y,x+7,y+7);
end;

begin
  graphdriver:=installuserdriver('svga256',nil);
  graphmode:=1;
  ch:=' ';
  number:=0;
  symmetry:=1;
  registerBGIDriver(@SVGADriver);
  initgraph(graphdriver,graphmode,'');
  writeln(graphresult);
  for i:=0 to 255 do setrgbpalette(i,palette[i*3+2],palette[i*3+3],palette[i*3+4]);
  directvideo:=false;
  mstatus(i,j);

  for i:=0 to 255 do
  begin
    setfillstyle(solidfill,i);
    bar(i*2,330,i*2+1,350);
  end;
  color:=7;
  if i=0 then exit;
  mhide;
  setfillstyle(solidfill,8);
  bar(0,0,7,7);
  getmem(p,imagesize(0,0,7,7));
  getmem(out,imagesize(450,20,450+xsize-1,20+ysize-1)+500);
  for i:=0 to maximages do
    getmem(an[i],imagesize(450,20+25*i,450+xsize-1,20+25*i+ysize-1)+500);
  getimage(0,0,7,7,p^);

  setfillstyle(solidfill,background);
  bar(0,280,640,290);
  if exist(filename) then
  begin
    assign(f,filename);
    reset(f,1);
    for i:=0 to maximages do
    begin
      blockread(f,xsize,2);
      blockread(f,ysize,2);
      bmxsize:=(((xsize-1) div 4)+1)*4;
      for k:=0 to (ysize-1) do
      for j:=0 to (bmxsize-1) do
      begin
        blockread(f,temp,1);
        putpixel(450+(i div 8)*40+j,k+20+25*(i and 7),temp);
        setfillstyle(solidfill,temp);
        bar(j*10,k*10,j*10+7,k*10+7);
      end;
    end;
  end;

{  bmxsize:=16;}
{   xsize:=16;}
{   ysize:=14; }
{  maximages:=9;}
{  xsize:=20; }

  mhlimit(0,10*xsize-1);
  mvlimit(0,10*ysize-1);
  mpos(i,j,k);
  setcolor(15);
  rectangle(0,0,10*xsize+1,10*ysize+1);
  x:=j div 10 * 10;
  y:=k div 10 * 10;
  oldx:=x;
  oldy:=y;
  setfillstyle(solidfill,4);
  repeat
    putimage(x,y,p^,xorput);
    j:=0;
    mbutpress(i,j,k,l);
    if i<>0 then
    begin
      if i=1 then
      begin
        putimage(x,y,p^,xorput);
        putbit(x,y,color);
        if (symmetry=2) or (symmetry=4) then putbit(xsize*10-10-x,y,color);
        if (symmetry>=3) then putbit(x,ysize*10-10-y,color);
        if symmetry=4 then putbit(xsize*10-10-x,ysize*10-10-y,color);
        putimage(x,y,p^,xorput);
      end
      else
      begin
        putimage(x,y,p^,xorput);
        putbit(x,y,background);
        if (symmetry=2) or (symmetry=4) then putbit(xsize*10-10-x,y,background);
        if (symmetry>=3) then putbit(x,ysize*10-10-y,background);
        if symmetry=4 then putbit(xsize*10-10-x,ysize*10-10-y,background);
        putimage(x,y,p^,xorput);
      end;
    end;
    mpos(i,j,k);
    oldx:=x;
    oldy:=y;
    putimage(oldx,oldy,p^,xorput);
    x:=j div 10 * 10;
    y:=k div 10 * 10;
    if keypressed then
    begin
      ch:=readkey;
      if ch='c' then
      begin
        setfillstyle(solidfill,0);
        bar(450,20+25*number,450+xsize-1,20+25*number+ysize-1);
      end;
      if (ch='+') or (ch='*') then
      begin
        putpixel(color*2,328,0);
        if (ch='+') and (color<255) then inc(color);
        if (ch='*') and (color<245) then inc(color,10);
        putpixel(color*2,328,15);
        setfillstyle(solidfill,color);
        bar(360,40,420,60);
      end;
      if (ch='-') or (ch='/') then
      begin
        putpixel(color*2,328,0);
        if (ch='-') and (color>0) then dec(color);
        if (ch='/') and (color>10) then dec(color,10);
        putpixel(color*2,328,15);
        setfillstyle(solidfill,color);
        bar(360,40,420,60);
      end;
      if ch='v' then begin
                       getimage(50,300,50+xsize-1,300+ysize-1,an[0]^);
                       for i:=1 to 10 do putimage(500,i*10,an[0]^,normalput);
                     end;
      if ch='s' then
      begin
        inc(symmetry);
        if symmetry=5 then symmetry:=1;
      end;
      if ch='r' then
      begin
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+450+40*(number div 8),j+20+25*(number and 7));
          putpixel(i+500,j+300,k);
        end;
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(xsize-i-1+500,j+300);
          putpixel(i+450+40*(number div 8),j+20+25*(number and 7),k);
        end;
      end;
      if ch=']' then
      begin
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+450+40*(number div 8),j+20+25*(number and 7));
          putpixel(i+500,j+300,k);
        end;
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+499,j+300);
          putpixel(i+450+40*(number div 8),j+20+25*(number and 7),k);
        end;
      end;
      if ch='[' then
      begin
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+450+40*(number div 8),j+20+25*(number and 7));
          putpixel(i+500,j+300,k);
        end;
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+501,j+300);
          putpixel(i+450+40*(number div 8),j+20+25*(number and 7),k);
        end;
      end;
      if ch=')' then
      begin
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+450+40*(number div 8),j+20+25*(number and 7));
          putpixel(i+500,j+300,k);
        end;
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+500,j+299);
          putpixel(i+450+40*(number div 8),j+20+25*(number and 7),k);
        end;
      end;
      if ch='(' then
      begin
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+450+40*(number div 8),j+20+25*(number and 7));
          putpixel(i+500,j+300,k);
        end;
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+500,j+301);
          putpixel(i+450+40*(number div 8),j+20+25*(number and 7),k);
        end;
      end;
      if ch='f' then
      begin
        getimage(450,20,450+xsize-1,20+ysize-1,an[0]^);
        for i:=1 to maximages do
          putimage(450,20+25*i,an[0]^,normalput);
      end;
      if ch='a' then
      begin
        for i:=0 to maximages do
        begin
          getimage(450+(i div 8)*40,20+25*(i and 7),450+xsize-1+(i div 8)*40,20+25*(i and 7)+ysize-1,an[i]^);
        end;
        i:=0;
        repeat
          i:=(i+1) mod (maximages+1);
          putimage(500,300,an[i]^,normalput);
          delay(100);
        until keypressed;
        readkey;
      end;
      if ch in ['0'..'9',',','.'] then
      begin
        if ch=',' then dec(number) else
          if ch='.' then inc(number) else
            val(ch,number,i);
        if number>maximages then number:=maximages;
        if number<0 then number:=0;
        for i:=0 to xsize do
        for j:=0 to ysize do
        begin
          k:=getpixel(i+450+40*(number div 8),j+20+25*(number and 7));

          setfillstyle(solidfill,k);
          bar(i*10,j*10,i*10+7,j*10+7);
        end;
      end;
    end;
  until (ch=#27) or (ch='Q');
  if ch='Q' then halt;
  repeat
    write(^g);
    ch:=readkey;
  until ch in ['y','n'];
  if ch='n' then halt;
  assign(f,filename2);
  ch:=readkey;
  rewrite(f,1);
  for i:=0 to maximages do
  begin
    blockwrite(f,xsize,2);
    blockwrite(f,ysize,2);
    for k:=0 to (ysize-1) do
    begin
      for j:=0 to (bmxsize-1) do
      begin
        temp:=getpixel(450+j+40*(i div 8),20+k+25*(i and 7));
        putpixel(450+j+40*(i div 8),20+k+25*(i and 7),31);
        blockwrite(f,temp,1);
      end;
    end;
  end;
  close(f);
end.


