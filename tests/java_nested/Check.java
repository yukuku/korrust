package nestedcases;
public class Check {
    static int assertions;
    static void equal(Object actual, Object expected) { assertions++; if (!actual.equals(expected)) throw new AssertionError(actual + " != " + expected); }
    public static void main(String[] args) throws Exception {
        equal(ReadsKt.publicRead(), 17);
        equal(ReadsKt.aliasRead(), 17);
        equal(ReadsKt.fullRead(), 33);
        equal(ReadsKt.wideRead(), 9223372036854775806L);
        equal(ReadsKt.labelRead(), "alpha");
        equal(ReadsKt.invoke(), 21);
        equal(ReadsKt.fieldPriority(), 91);
        equal(ReadsKt.classifierPriority(), 50);
        equal(ReadsKt.lexicalShadow(), 71);
        equal(library.SamePackageKt.samePackage(), 29);
        equal(library.AliasPackageKt.samePackageAlias(), 29);
        library.Warehouse.Tools.total = 80;
        library.Warehouse.Tools.label = "beta";
        library.Warehouse.changePackage(63);
        library.Probe shadow = library.Warehouse.Shadow;
        shadow.value = 92;
        library.Warehouse.Shadow.class.getField("value").setInt(null, 65);
        equal(ReadsKt.publicRead(), 80);
        equal(ReadsKt.fieldPriority(), 92);
        equal(ReadsKt.classifierPriority(), 65);
        equal(ReadsKt.aliasRead(), 80);
        equal(ReadsKt.labelRead(), "beta");
        equal(ReadsKt.invoke(), 84);
        equal(library.SamePackageKt.samePackage(), 63);
        equal(library.AliasPackageKt.samePackageAlias(), 63);
        library.Warehouse.Tools.label = "A\uD83D\uDE00\uD800";
        equal(ReadsKt.labelRead(), "A\uD83D\uDE00\uD800");
        library.Warehouse.Tools.label = null;
        try { ReadsKt.labelRead(); throw new AssertionError("missing platform assertion"); } catch (NullPointerException expected) { assertions++; }
        System.out.println("OK " + assertions);
    }
}
