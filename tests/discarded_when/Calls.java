public final class Calls {
 static void eq(Object a, Object b) { if (!java.util.Objects.equals(a,b)) throw new AssertionError(a+" != "+b); }
 public static void main(String[] args) {
  EffectsKt.resetTrace();
  for(int i=0;i<4;i++) EffectsKt.discardWhen(i);
  eq(EffectsKt.readTrace(),"SU");
  EffectsKt.discardBound(1); eq(EffectsKt.readTrace(),"SU1");
  EffectsKt.lambdaWhen(0).invoke(); EffectsKt.lambdaWhen(1).invoke(); eq(EffectsKt.readTrace(),"SU1L");
  EffectsKt.orderedWhen(1); EffectsKt.orderedWhen(2); eq(EffectsKt.readTrace(),"SU1L1A2");
  eq(EffectsKt.branchReturn(0),17); eq(EffectsKt.branchReturn(1),19); eq(EffectsKt.readTrace(),"SU1L1A2B");
  System.out.println("OK");
 }
}
