import overrideoperators.UseKt;
public class Calls { public static void main(String[] args) {
 for(int n:new int[]{Integer.MIN_VALUE,64,65,90,91,Integer.MAX_VALUE}) {
  boolean expected=n>=65&&n<=90;
  if(UseKt.present(n)!=expected || UseKt.letters(n)!=expected)throw new AssertionError(n);
 }
 if(!UseKt.accepts("yes")||UseKt.accepts("no")||!UseKt.lookup("key").equals("key"))throw new AssertionError("generic dispatch");
 System.out.println("OK");
} }
