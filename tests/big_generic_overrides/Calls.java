import genericdispatch.*;
public class Calls {
 static void check(Api<String> api, String expected) {
  if (!api.echo(7).equals(7)) throw new AssertionError("echo");
  Named n=new Named("bound"); if(api.bounded(n)!=n) throw new AssertionError("bound");
  if(!api.choose(7).equals(expected)) throw new AssertionError("choose");
  if(!api.take("seed",42).equals(42)) throw new AssertionError("take");
 }
 public static void main(String[] args) {
  check(new Impl<String>("generic"),"generic"); check(new TextApi(),"class");
  if(!Singleton.INSTANCE.echo("object").equals("object")) throw new AssertionError();
  Base<String> child=new Child(); if(!child.echo(19).equals(19)||!child.choose(7).equals("child"))throw new AssertionError("child");
  if(!OverridesKt.through(new TextApi()).equals("class!")) throw new AssertionError("source dispatch");
  Api<Integer> numbers=new IntApi(); if(numbers.choose("x")!=17 || !numbers.take(3,"ok").equals("ok"))throw new AssertionError("primitive");
  try { ((Api)new TextApi()).take(12,"x"); throw new AssertionError("missing bridge cast"); } catch(ClassCastException expected) {}
  try { ((Api)new TextApi()).take(1,"bad"); throw new AssertionError("bridge cast"); } catch(ClassCastException expected) {}
  try { numbers.take(null,"bad"); throw new AssertionError("bridge null"); } catch(NullPointerException expected) {}
  System.out.println("OK");
 }
}
