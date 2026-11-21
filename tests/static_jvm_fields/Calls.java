package staticprobe;
import java.lang.reflect.*;
import java.util.Objects;
public class Calls {
 static void eq(Object expected,Object actual) { if(!Objects.equals(expected,actual)) throw new AssertionError(expected+" != "+actual); }
 public static void main(String[] args) throws Exception {
  eq("A",Host.first); eq("ABCD",Trace.INSTANCE.getText()); eq("B",Host.Companion.getOrdinary()); eq("ABC",Host.Companion.own());
  eq("A",Host.Companion.deferred().invoke());
  Host.mutable="M"; eq("ABM",Host.Companion.own());
  eq("E",Global.value); eq("ABCDEF",Trace.INSTANCE.getText());
  Global.number=9; eq("EF9",Global.INSTANCE.own());
  eq("AE",ReadKt.read()); eq("A",ReadKt.qualified()); ReadKt.update(); eq("X",Host.mutable); eq(9,Global.number);
  for(String name:new String[]{"getFirst","getMutable","setMutable"}) for(Method m:Host.Companion.getClass().getDeclaredMethods()) if(m.getName().equals(name)) throw new AssertionError(name);
  if(!Modifier.isPublic(Host.class.getField("first").getModifiers()) || !Modifier.isFinal(Host.class.getField("first").getModifiers()) || Modifier.isFinal(Host.class.getField("mutable").getModifiers())) throw new AssertionError();
  eq(4, Cache.EMPTY.getValue()); eq("method",Cache.Companion.getEMPTY());
  Cache.boxed = 6; eq(Integer.valueOf(6),Cache.boxed); eq(java.util.Arrays.asList("a","b"),Cache.names);
  eq(8L,Cache.number); eq(2.5,Cache.decimal);
  Aliased.text="updated"; eq("updated",Aliased.text);
  if(Cache.class.getField("EMPTY").getDeclaringClass()!=Cache.class || Global.class.getField("value").getDeclaringClass()!=Global.class) throw new AssertionError();
  if(Cache.class.getField("boxed").getType()!=Integer.class || Cache.class.getField("number").getType()!=long.class || Cache.class.getField("decimal").getType()!=double.class) throw new AssertionError();
  if(!Cache.class.getField("names").getGenericType().getTypeName().equals("java.util.List<java.lang.String>")) throw new AssertionError();
  eq(3,staticprobe.shadow.Shadowed.INSTANCE.getValue());
  if(!Modifier.isPrivate(staticprobe.shadow.Shadowed.class.getDeclaredField("value").getModifiers())) throw new AssertionError();
  System.out.println("OK");
 }
}
