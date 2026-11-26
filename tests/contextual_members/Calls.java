import contextcalls.*;

public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    public static void main(String[] args) {
        Service service = new Service();
        Holder holder = new Holder();
        holder.fill(1); check(holder.getResult().code == 1);
        check(holder.implicit(3).code == 3 && holder.throughThis(4).code == 4);
        check(holder.mixed().code == 1 && EffectsKt.mixed(service).code == 1);
        check(holder.zero().code == 1 && holder.overload().code == 4);
        check(holder.local(5).code == 5 && holder.nullable(0) == null);
        check(EffectsKt.explicit(service, 3).code == 3);
        check(EffectsKt.broad(service).code == 1 && EffectsKt.assign(service).code == 1);
        check(EffectsKt.<Leaf>parameterized(service).code == 1);
        Service.log = "";
        check(EffectsKt.computed().code == 3 && Service.log.equals("RAL"));
        try { holder.implicit(0); throw new AssertionError("missing null check"); }
        catch (NullPointerException expected) { }
        try { EffectsKt.explicit(service, 2); throw new AssertionError("missing cast"); }
        catch (ClassCastException expected) { }
        System.out.println("OK");
    }
}
