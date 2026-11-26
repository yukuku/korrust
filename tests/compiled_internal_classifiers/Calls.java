package friends.consumer;
public class Calls {
    public static void main(String[] args) {
        if (ConsumerKt.total() != 41 || ExplicitKt.explicit() != 9 || friends.library.SamePackageKt.samePackage() != 8) throw new AssertionError("friend classifier calls");
        if (new PublicCounter().value() != 23 || PublicInheritanceKt.publicInheritance() != 60 || friends.library.ShadowKt.shadow() != 12) throw new AssertionError("public inheritance / shadowing");
        if (PrivateAliasKt.privateAlias() != 2) throw new AssertionError("private alias");
        System.out.println("internal-classifiers-ok");
    }
}
