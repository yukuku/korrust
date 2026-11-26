package qualifiedcheck;

import qv.p.State;
import qv.p.Root;
import qualifiedconsumer.UsesKt;

public class Calls {
    public static void main(String[] args) {
        if (UsesKt.javaWork() != 88 || State.count != 10) throw new AssertionError("static writes");
        if (UsesKt.kotlinWork() != 817) throw new AssertionError("objects and companions");
        Root root = new Root();
        if (UsesKt.parameterShadow(root) != 32 || root.p.State.count != 33 || State.count != 10) throw new AssertionError("parameter shadow");
        if (UsesKt.captureShadow(root) != 36 || root.p.State.count != 36) throw new AssertionError("capture shadow");
        if (new qualifiedconsumer.MemberShadow(root).work() != 41 || root.p.State.count != 41) throw new AssertionError("member shadow");
        if (new qualifiedconsumer.InheritedShadow(root).work() != 42 || root.p.State.count != 42 || State.count != 10) throw new AssertionError("inherited shadow");
        if (qualifiedalias.AliasKt.work() != 54 || State.ROOT.p.State.count != 54 || State.count != 10) throw new AssertionError("alias shadow");
        if (qualifiedpropertyalias.PropertyAliasKt.work() != 64 || qv.kotlin.DefinitionsKt.getRoot().p.State.count != 64 || State.count != 10) throw new AssertionError("property alias shadow");
        if (UsesKt.nestedWork() != 71) throw new AssertionError("same package nested classifier");
        System.out.println("qualified-values-ok");
    }
}
