import interfacebodies.*;

public class Calls {
    private static void check(boolean result) { if (!result) throw new AssertionError(); }
    public static void main(String[] args) {
        check(BodiesKt.run().equals("ABCDE6FGHIJ!"));
        Generic<String> generic = new Text();
        Generic<Integer> number = new IntDefault();
        Ordinary ordinary = new Plain();
        check(generic.own() == generic);
        check(generic.identity("java").equals("java"));
        check(generic.identity(46) == 46);
        check(generic.identity(null) == null);
        check(generic.echo(null) == null);
        check(number.echo(42) == 42);
        check(new IntDefault().echo(44) == 44);
        check(generic.bounded(new Name("bound")).text().equals("bound"));
        check(generic.consume(1, "context").equals("context"));
        check(generic.label(new Name("label-body")).equals("label-body"));
        check(ordinary.label(new Name("ordinary-body")).equals("ordinary-body"));
        check(ordinary.relay("plain").equals("plain"));
        check(Generic.DefaultImpls.identity(generic, "helper").equals("helper"));
        check(Generic.DefaultImpls.echo(generic, "helper").equals("helper"));
        check(Ordinary.DefaultImpls.identity(ordinary, null) == null);
        check(new OverrideDefault().relay("override").equals("override"));
        check(new RefinedImpl().relay("refined").equals("refined"));
        check(Named.INSTANCE.identity("named").equals("named"));
        check(Named.INSTANCE.bounded(new Name("object-bound")).text().equals("object-bound"));
        check(Empty.INSTANCE.identity("empty").equals("empty"));
        check(Empty.INSTANCE.relay("empty-relay").equals("empty-relay"));
        check(BodiesKt.anonymous().identity("anonymous").equals("anonymous"));
        check(BodiesKt.anonymous().identity(null) == null);
        check(BodiesKt.anonymous() != BodiesKt.anonymous());
        check(BodiesKt.anonymous().bounded(new Name("anonymous-bound")).text().equals("anonymous-bound"));
        check(new ChildText().echo("child").equals("child"));
        check(new ChildNumber().echo(43) == 43);
        check(new Specialized().consume(1, "special").equals("special"));
        check(new BoundedImpl().same(new Name("class-bound")).text().equals("class-bound"));
        check(new PropertyText("property").getCopy().equals("property"));
        check(generic.closure("lambda").invoke().equals("lambda"));
        check(Child.DefaultImpls.echo(new ChildText(), "child-helper").equals("child-helper"));
        check(StringChild.DefaultImpls.echo(new Specialized(), "special-helper").equals("special-helper"));
        System.out.println("OK");
    }
}
