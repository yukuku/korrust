package verify;
public class JavaCheck {
 static int checks;
 static void truth(boolean value) { checks++; if(!value)throw new AssertionError("check "+checks); }
 public static void main(String[] a) {
 truth(probe.ReifiedKt.result()==18); truth(probe.ReifiedKt.yes("x")); truth(!probe.ReifiedKt.yes(1)); truth(probe.ReifiedKt.intYes(1)); truth(!probe.ReifiedKt.intYes("x")); truth(probe.ReifiedKt.nullable(null)); truth(!probe.ReifiedKt.yes(null));
 truth(probe.ReifiedKt.str("x").equals("x")); truth(probe.ReifiedKt.strSafe(1)==null); truth(probe.ReifiedKt.cls().equals(kotlin.jvm.JvmClassMappingKt.getKotlinClass(Integer.TYPE)));
 try{probe.ReifiedKt.str(null);throw new AssertionError();}catch(NullPointerException expected){checks++;}
 try{probe.ReifiedKt.str(1);throw new AssertionError();}catch(ClassCastException expected){checks++;}
 truth(probe.ReifiedKt.physicalInt()==Integer.class); truth(probe.ReifiedKt.physicalNullable()==Integer.class);
 truth(lambdas.EffectsKt.direct()==22); truth(lambdas.EffectsKt.nonlocal()==5); truth(lambdas.EffectsKt.cross()==23); truth(lambdas.EffectsKt.no()==8); truth(lambdas.EffectsKt.cleanup()==5); truth(lambdas.EffectsKt.getEffects()==1);
 truth(lambdas.EffectsKt.captured()==1308); truth(lambdas.EffectsKt.cleanupLambda()==1); truth(lambdas.EffectsKt.getEffects()==110);
 truth(lambdas.EffectsKt.generic().equals("v3")); truth(lambdas.EffectsKt.genericReturn()==5); truth(lambdas.EffectsKt.privateCall()==14);
 truth(lambdas.EffectsKt.overloaded()==8);
 truth(forward.ForwardKt.answer()==15); truth(forward.ForwardKt.yes("x")); truth(!forward.ForwardKt.yes(2));
 truth(ordering.OrderingKt.property()==8);
 truth(ordering.OrderingKt.order()==9); truth(ordering.OrderingKt.getTrace()==2145); truth(ordering.OrderingKt.exception()==9); truth(ordering.OrderingKt.getTrace()==7); truth(ordering.OrderingKt.throwing()==12);
 for(int x:new int[]{0,1,-1,32767,32768,-32768,-32769,65535,65536,Integer.MIN_VALUE,Integer.MAX_VALUE}) {
 short s=(short)x; truth(shorts.ShortsKt.narrow(x)==s); truth(shorts.ShortsKt.widen(s)==s); truth(shorts.ShortsKt.longValue(s)==s); truth(shorts.ShortsKt.floatValue(s)==s); truth(shorts.ShortsKt.doubleValue(s)==s); truth(shorts.ShortsKt.narrowLong(x)==s); truth(shorts.ShortsKt.narrowDouble(x)==s); truth(shorts.ShortsKt.echo(s).shortValue()==s); truth(shorts.ShortsKt.bound(s).equals(kotlin.jvm.JvmClassMappingKt.getKotlinClass(Short.TYPE)));
 }
 truth(probe.ReifiedKt.shortYes((short)2)); truth(!probe.ReifiedKt.shortYes(2)); truth(probe.ReifiedKt.shortCast((short)-32768)==-32768); truth(probe.ReifiedKt.shortSafe(2)==null); truth(probe.ReifiedKt.shortSafe((short)32767).shortValue()==32767);
 for(long value:new long[]{Long.MIN_VALUE,Long.MAX_VALUE,65536L,-65537L}) truth(shorts.ShortsKt.narrowLong(value)==(short)value);
 for(double value:new double[]{Double.NaN,Double.NEGATIVE_INFINITY,Double.POSITIVE_INFINITY,-32768.99,32768.99,-1e100,1e100}) truth(shorts.ShortsKt.narrowDouble(value)==(short)value);
 truth(shorts.ShortsKt.echo(null)==null); truth(shorts.ShortsKt.literal()==-32768); truth(shorts.ShortsKt.classifier().equals(kotlin.jvm.JvmClassMappingKt.getKotlinClass(Short.TYPE)));
 System.out.println("checks="+checks);
 }
}
