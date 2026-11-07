program testcompander;
uses crt;
var r:real;
    i,j:integer;
const Maximum=1024;

begin
  writeln('Go');
  for J:=0 to 3 do
  begin
    for i:=0 to 127 do write(55*ln((i+128*j)/55+1):5:0);
    readkey;
  end;
end.
