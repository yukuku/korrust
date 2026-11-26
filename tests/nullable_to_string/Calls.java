import stringify.Probe;
public final class Calls {
 private static void equal(Object actual, Object expected) { if (!java.util.Objects.equals(actual, expected)) throw new AssertionError(actual + " != " + expected); }
 public static void main(String[] args) {
  equal(EffectsKt.nullable(null), "null"); equal(EffectsKt.literalNull(), "null");
  equal(EffectsKt.nullableString(null), "null"); equal(EffectsKt.nullableString("abc"), "abc");
  equal(EffectsKt.number(null), "null"); equal(EffectsKt.number(42), "42");
  equal(EffectsKt.bool(null), "null"); equal(EffectsKt.bool(true), "true");
  equal(EffectsKt.character(null), "null"); equal(EffectsKt.character('x'), "x");
  char[] chars = new char[] {'a','b'}; equal(EffectsKt.chars(chars), chars.toString()); equal(EffectsKt.chars(null), "null");
  String[] strings = new String[] {"a"}; equal(EffectsKt.references(strings), strings.toString()); equal(EffectsKt.references(null), "null");
  equal(EffectsKt.generic(null), "null"); equal(EffectsKt.generic("generic"), "generic");
  Probe probe = new Probe(1); equal(EffectsKt.nullable(probe), "probe"); equal(probe.calls, 1);
  Probe nullText = new Probe(2); equal(EffectsKt.nullable(nullText), null); equal(nullText.calls, 1);
  Probe throwing = new Probe(3); try { EffectsKt.nullable(throwing); throw new AssertionError(); } catch (IllegalStateException expected) { equal(expected.getMessage(), "stringify"); } equal(throwing.calls, 1);
  Probe.gets = 0; equal(EffectsKt.next(0), "null"); equal(Probe.gets, 1); equal(EffectsKt.next(1), "probe"); equal(Probe.gets, 2);
  equal(EffectsKt.safe(null), null); equal(EffectsKt.bound(null), "null");
  equal(localshadow.LocalKt.local(null), "local"); equal(importedshadow.ImportedKt.imported(null), "imported");
  equal(aliasshadow.AliasKt.aliased(null), "null");
  Probe member = new Probe(1); equal(membershadow.MemberKt.member(member), "probe"); equal(member.calls, 1); equal(membershadow.MemberKt.nullableMember(null), "extension");
  System.out.println("OK");
 }
}
