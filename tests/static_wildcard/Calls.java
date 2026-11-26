package star.consumer;
public class Calls {
    public static void main(String[] args) {
        if (ValuesKt.integer() != 15 || !ValuesKt.text().equals("first:value") || ValuesKt.constant() != 13 || ValuesKt.inferred() != 17) throw new AssertionError("basic overloads");
        if (ValuesKt.captured() != 19 || ValuesKt.reference() != 21 || ValuesKt.nested() != 29) throw new AssertionError("callable imports");
        if (ValuesKt.field() != 3 || ValuesKt.mutate() != 4 || ValuesKt.field() != 4) throw new AssertionError("mutable field");
        if (OverloadsKt.fromFirst() != 15 || OverloadsKt.fromSecond() != 25 || OverloadsKt.separate() != 43) throw new AssertionError("merged candidates");
        if (ExplicitKt.selected() != 119 || AliasKt.alias() != 103 || DuplicateKt.duplicate() != 23) throw new AssertionError("explicit alias duplicate");
        if (InheritedDuplicateKt.inheritedDuplicate() != 71) throw new AssertionError("inherited duplicate");
        if (InheritedKt.inheritedMethod() != 34 || InheritedKt.inheritedField() != 37 || star.local.SourcePriorityKt.sourcePriority() != 1002) throw new AssertionError("inheritance source");
        System.out.println("static-wildcard-ok");
    }
}
