import variance.Bags;
public final class Calls {
    public static void main(String[] args) {
        Bags holder = new Bags();
        java.util.List<String> items = new java.util.ArrayList<>(); items.add("a");
        EffectsKt.fields(holder, items);
        if ((Object) holder.field != items || (Object) holder.getValue() != items || EffectsKt.readonly(items) != 1 || EffectsKt.projectedSame(items) != 1 || EffectsKt.flexibleSame() != 0 || EffectsKt.nullable() != -1) throw new AssertionError();
        Bags.calls = 0;
        if (EffectsKt.once() != 0 || Bags.calls != 1) throw new AssertionError("argument evaluated more than once");
        System.out.println("OK");
    }
}
