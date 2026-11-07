unit filexist;
interface
function exist(filename:string):boolean;
implementation
function exist(filename:string):boolean;
var fil:file;
    a:integer;
begin
  assign(fil,filename);
  {$I-}
  reset(fil);
  {$I+}
  a:=ioresult;
  exist:=(a=0);
  if a=0 then close(fil);
end;

end.