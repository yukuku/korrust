import updates.*;
public class Check {
  static int checks;
  static void eq(Object actual, Object expected) { checks++; if (!java.util.Objects.equals(actual, expected)) throw new AssertionError(actual + " != " + expected); }
  static void reset(int failure) { Events.INSTANCE.setTrace(""); Events.INSTANCE.setFailure(failure); }
  static void trace(String expected) { eq(Events.INSTANCE.getTrace(), expected); }
  static void failure(Runnable action, String message, String expected) {
    try { action.run(); throw new AssertionError("missing " + message); }
    catch (IllegalStateException error) { eq(error.getMessage(), message); }
    trace(expected);
  }
  public static void main(String[] args) {
    Holder h = new Holder(new Counter(10));
    reset(0); Counter old = OperatorsKt.post(h); eq(old.getValue(), 10); eq(h.getSaved().getValue(), 11); trace("hg+s");
    reset(0); eq(OperatorsKt.down(h).getValue(), 11); trace("hg-s");
    reset(0); OperatorsKt.compound(h); eq(h.getSaved().getValue(), 12); trace("hgrps");
    for (int f = 1; f <= 4; f++) {
      reset(f); failure(() -> OperatorsKt.compound(h), new String[]{"", "get", "rhs", "op", "set"}[f], new String[]{"", "hg", "hgr", "hgrp", "hgrps"}[f]); eq(h.getSaved().getValue(), 12);
    }
    for (int f : new int[]{1,3,4}) {
      reset(f); failure(() -> OperatorsKt.post(h), new String[]{"", "get", "", "op", "set"}[f], new String[]{"", "hg", "", "hg+", "hg+s"}[f]); eq(h.getSaved().getValue(), 12);
    }
    Indexed index = new Indexed(new Counter(20));
    reset(0); eq(OperatorsKt.indexPost(index).getValue(), 20); eq(index.getSaved().getValue(), 21); trace("hijg+s");
    reset(0); OperatorsKt.indexCompound(index); eq(index.getSaved().getValue(), 23); trace("hijgrps");
    for (int f = 1; f <= 4; f++) {
      reset(f); failure(() -> OperatorsKt.indexCompound(index), new String[]{"", "get", "rhs", "op", "set"}[f], new String[]{"", "hijg", "hijgr", "hijgrp", "hijgrps"}[f]); eq(index.getSaved().getValue(), 23);
    }
    reset(5); failure(() -> OperatorsKt.indexCompound(index), "index", "hi"); eq(index.getSaved().getValue(),23);
    reset(6); failure(() -> OperatorsKt.indexPost(index), "wide", "hij"); eq(index.getSaved().getValue(),23);
    reset(7); failure(() -> OperatorsKt.compound(h), "receiver", "h"); eq(h.getSaved().getValue(),12);
    reset(0); Mutating mut = new Mutating(5); OperatorsKt.assign(new ReadOnly(mut)); eq(mut.getValue(), 7); trace("hra");
    reset(0); Dual dual = new Dual(3); OperatorsKt.dualProperty(new DualHolder(dual)); eq(dual.getValue(),5); trace("ra");
    reset(0); OperatorsKt.dualIndex(new DualIndex(dual)); eq(dual.getValue(),7); trace("jgra");
    reset(0); Numeric numeric = new Numeric(4); OperatorsKt.primitiveProperty(numeric); eq(numeric.getSaved(),8); trace("grs");
    reset(0); eq(OperatorsKt.primitivePost(numeric),8); eq(numeric.getSaved(),9); trace("gs");
    int[] ints = {Integer.MAX_VALUE}; reset(0); eq(OperatorsKt.arrayPost(ints), Integer.MAX_VALUE); eq(ints[0], Integer.MIN_VALUE); trace("i");
    long[] longs = {Long.MAX_VALUE}; reset(0); OperatorsKt.arrayCompound(longs); eq(longs[0], Long.MIN_VALUE + 1); trace("ir");
    Counter[] counters = {new Counter(8)}; reset(0); eq(OperatorsKt.arrayCounter(counters).getValue(), 9); eq(counters[0].getValue(), 9); trace("i+");
    reset(0); eq(OperatorsKt.local(), 243); kotlin.jvm.functions.Function0<Integer> captured = OperatorsKt.captured(); eq(captured.invoke(),23); eq(captured.invoke(),34); eq(OperatorsKt.subtype(), 8);
    reset(0); eq(OperatorsKt.extension(),22); eq(OperatorsKt.inherited(),5); eq(OperatorsKt.dispatch(),78); eq(OperatorsKt.numbers(),-32882L);
    reset(0); eq(OperatorsKt.rebind(new Holder(new Counter(5))),100007); trace("grps");
    reset(0); eq(OperatorsKt.narrow(new NarrowIndex(new Counter(4))),45);
    eq(ImportedKt.compiled(new foreign.Bucket(new foreign.Meter(3))),36); eq(ImportedKt.compiledExtension(),44);
    System.out.println("OK " + checks);
  }
}
