import methodgenerics.*;
public class Calls {
    static class Api implements GenericApi<String> {
        public <T extends Label> T identity(T value) { return value; }
        public <T> String choose(T value) { return "chosen"; }
    }
    public static void main(String[] args) throws Exception {
        Generic<String> generic = new Generic<>("seed");
        Named named = new Named("OK");
        if (!new Generic<String>("secondary", 0).getSeed().equals("secondary")) throw new AssertionError();
        if (generic.echo(17) != 17 || generic.bounded(named) != named || !generic.text(named).equals("OK")) throw new AssertionError();
        if (!generic.choose(18).equals("seed") || !generic.second(7, "second").equals("second")) throw new AssertionError();
        if (!generic.first(java.util.Arrays.asList("first")).equals("first")) throw new AssertionError();
        if (!generic.transform(named, n -> n.text()).equals("OK")) throw new AssertionError();
        if (Helpers.bounded(named) != named || Helpers.INSTANCE.echo(named) != named) throw new AssertionError();
        Api api = new Api();
        if (UseKt.through(api, named) != named || !UseKt.chooseThrough(api).equals("chosen")) throw new AssertionError();
        if (!UseKt.localCalls().equals("OK!OK")) throw new AssertionError();
        Cov<Final> cf = new Cov<>(new Final());
        Cov<Open> co = new Cov<>(new Open());
        if (UseKt.p1(cf) != 1 || UseKt.p3(co) != 3 || UseKt.p5(new Con<Object>()) != 5) throw new AssertionError();
        if (UseKt.p10(new Cov<>(new Con<Object>())) != 10 || new WildMethods(cf).take(new Con<Object>()) != 17) throw new AssertionError();
        if (new WildDefaults().take(cf) != 18 || WildDefaults.takeStatic(cf) != 19 || new WildDefaultImpl().take(co) != 20) throw new AssertionError();
        java.lang.reflect.TypeVariable<?> bound = Generic.class.getDeclaredMethod("bounded", Label.class).getTypeParameters()[0];
        if (bound.getBounds()[0] != Label.class || !Generic.class.getDeclaredMethod("bounded", Label.class).getGenericReturnType().equals(bound)) throw new AssertionError();
        System.out.println("OK");
    }
}
