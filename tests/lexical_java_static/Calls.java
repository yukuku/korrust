import fields.Base;
import fieldsuse.Child;
public final class Calls {
 private static void equal(Object actual, Object expected) { if (!java.util.Objects.equals(actual, expected)) throw new AssertionError(actual + " != " + expected); }
 public static void main(String[] args) {
  Child child = new Child(); Base.counter = 5;
  equal(child.direct(), 17); equal(child.captured(), 22); equal(child.sam(), 22);
  Base.counter = 8; equal(child.captured(), 25); equal(child.sam(), 25);
  equal(child.local(), 3); equal(new fieldsuse.Own().own(), 41);
  equal(new importedfield.Imported().code(), 17); equal(new fieldsuse.Grand().inherited(), 17);
  equal(interfacestatic.InterfaceKt.importedMark(), 19); equal(new fieldsuse.GetterChild().inheritedGetter(), 43); equal(new fieldsuse.StoredChild().inheritedStored(), 47);
  Base.label = "changed"; equal(child.labelValue(), "changed");
  Base.label = null; try { child.labelValue(); throw new AssertionError(); } catch (NullPointerException expected) {}
  equal(new packagefield.PackageChild().code(), 17); equal(new packagefield.CompanionChild().code(), 61);
  equal(new packagevalue.ValueChild().code(), 17); equal(new packagevalue.Scoped().code(), 71); equal(new packagevalue.Exposed().code(), 73);
  System.out.println("OK");
 }
}
