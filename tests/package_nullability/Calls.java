package defaults.consumer;
public class Calls {
 public static void main(String[] args) {
  if (!ConsumerKt.nonNull().equals("ok") || ConsumerKt.nullableValue()!=null || ConsumerKt.explicitNullable()!=null || !ConsumerKt.explicitRequired().equals("required") || ConsumerKt.platformResult()!=null || ConsumerKt.values()!=8 || ConsumerKt.fields()!=7 || ConsumerKt.nullableField()!=null || ConsumerKt.nullableElement()!=null || ConsumerKt.nullableArrayElement()!=null || !ConsumerKt.qualifierAlias().equals("alias") || ConsumerKt.kotlinMetadataWins()!=null) throw new AssertionError();
  try { BrokenKt.brokenRequired(); throw new AssertionError("required return"); } catch (NullPointerException expected) {}
  try { BrokenKt.brokenInferred(); throw new AssertionError("inferred local"); } catch (NullPointerException expected) {}
  if (BrokenKt.brokenNullable()!=null || !BrokenKt.brokenElvis().equals("fallback") || BrokenKt.brokenSafe()!=null || BrokenKt.brokenDiscarded()!=3) throw new AssertionError("Java boundary context/evaluation order");
  try { BrokenKt.requiredField(); throw new AssertionError("required field"); } catch (NullPointerException expected) {}
  if (BrokenKt.nullableFieldContext()!=null || BrokenKt.brokenArrayReceiver()!=1 || BrokenKt.brokenNodeReceiver()!=1 || BrokenKt.brokenListReceiver()!=1 || BrokenKt.brokenExtensionReceiver()!=0 || BrokenKt.nullableExtensionReceiver()!=1) throw new AssertionError("Java field/array boundary");
  if (WarningsKt.warningNullable()!=null || WarningsKt.warningInferred()!=1 || WarningsKt.warningGlobal()!=null) throw new AssertionError("warning flexibility");
  try { WarningsKt.warningRequired(); throw new AssertionError("warning required context"); } catch (NullPointerException expected) {}
  System.out.println("package-defaults-ok");
 }
}
