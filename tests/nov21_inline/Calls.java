package consumer;
import roots.*;
public final class Calls {
  static int checks;
  static void equal(Object expected, Object actual) { checks++; if (!java.util.Objects.equals(expected, actual)) throw new AssertionError(expected + " != " + actual); }
  public static void main(String[] args) {
    Leaf v = new Leaf();
    equal("leaf", ValuesKt.describe(v)); equal(97, ValuesKt.fingerprint(v));
    equal(true, ValuesKt.equivalent(v, v)); equal(false, ValuesKt.equivalent(v, null));
    equal(114, ValuesKt.inherited(v)); equal("pre:null:post", ValuesKt.ref(null));
    equal("pre:leaf:post", ValuesKt.ref(v)); equal("[leaf]", ValuesKt.nullable(v)); equal("[null]", ValuesKt.nullable(null));
    equal("[-2147483648]", ValuesKt.primitive(Integer.MIN_VALUE));
    char[] chars = {'a', '\ud83d', '\ude00'}; int[] ints = {1,2}; String[] arr = {"a", null};
    equal(chars.toString(), ValuesKt.chars(chars)); equal(ints.toString(), ValuesKt.ints(ints)); equal(arr.toString(), ValuesKt.array(arr));
    Effects.log = ""; equal("value/value", ValuesKt.evaluate()); equal("ESES", Effects.log);
    Effects.log = ""; try { ValuesKt.fails(); throw new AssertionError(); } catch (IllegalStateException x) { equal("string-failure", x.getMessage()); } equal("BX", Effects.log);
    Object invalid = new Object() { public String toString() { return null; } }; equal("pre:null:post", ValuesKt.ref(invalid));
    equal(String[].class, ValuesKt.vacantStrings(2).getClass()); equal(2, ValuesKt.vacantStrings(2).length);
    equal(Integer[].class, ValuesKt.vacantInts(2).getClass()); equal(null, ValuesKt.vacantInts(1)[0]);
    equal(Long[].class, ValuesKt.vacantLongs(0).getClass()); equal(int[][].class, ValuesKt.vacantArrays(1).getClass());
    equal(String[][].class, ValuesKt.vacantNested(1).getClass()); equal(Leaf[].class, ValuesKt.vacantLeaf(1).getClass());
    try { ValuesKt.vacantStrings(-1); throw new AssertionError(); } catch (NegativeArraySizeException x) { checks++; }
    java.util.List<String> items = java.util.Arrays.asList("a", "b");
    equal(String[].class, ValuesKt.gathered(items).getClass()); equal("[a, b]", java.util.Arrays.toString(ValuesKt.gathered(items)));
    equal("[a, b]", java.util.Arrays.toString(ValuesKt.directGathered(items)));
    equal(0, ValuesKt.directGathered(java.util.Collections.emptyList()).length);
    equal(Boolean[].class, ValuesKt.vacantBooleans(1).getClass()); equal(Character[].class, ValuesKt.vacantChars(1).getClass());
    equal(Byte[].class, ValuesKt.vacantBytes(1).getClass()); equal(Short[].class, ValuesKt.vacantShorts(1).getClass());
    equal(Float[].class, ValuesKt.vacantFloats(1).getClass()); equal(Double[].class, ValuesKt.vacantDoubles(1).getClass());
    equal(String[].class, ValuesKt.vacantNullable(1).getClass());
    Effects.log = ""; equal(3, ValuesKt.vacancyEffect().length); equal("N", Effects.log);
    Effects.log = ""; try { ValuesKt.vacancyThrow(); throw new AssertionError(); } catch (IllegalArgumentException x) { equal("size-failure", x.getMessage()); } equal("F", Effects.log);
    equal("label:ok", ValuesKt.valueClassTemplate("ok")); equal(Label[].class, ValuesKt.vacantLabels(1).getClass());
    equal(42, ValuesKt.hiddenParent(new Visible()));
    equal(kotlin.jvm.internal.Reflection.getOrCreateKotlinClass(String.class).toString(), ValuesKt.classTemplate()); equal("kotlin.Unit", ValuesKt.unitObjectTemplate());
    equal("leaf", ValuesKt.genericTemplate(v)); equal("null", ValuesKt.genericTemplate(null)); equal("[a, b]", ValuesKt.listTemplate(items));
    System.out.println("checks=" + checks);
  }
}
