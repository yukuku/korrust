package newinterop;

public final class Calls {
    private static void equal(Object expected, Object actual) {
        if (!expected.equals(actual)) throw new AssertionError(expected + " != " + actual);
    }

    public static void main(String[] args) {
        equal("i2sai3:6:17:1", newinterop.use.UseKt.staticCalls());
        Bean.log = "";
        equal("child:base:child:next:true:url:3:read", newinterop.use.UseKt.properties(new ChildBean()));
        equal("get;set(next);get;", Bean.log);
        Bean.log = "";
        Factory.calls = 0;
        equal("child:new", newinterop.use.UseKt.evaluationOrder());
        equal("receiver;argument;set(new);receiver;get;", Bean.log);
        equal(2, Factory.calls);
        if (newinterop.use.UseKt.nullable(null) != null) throw new AssertionError("nullable getter was invoked");
        equal("child:new", newinterop.use.UseKt.nullable(Factory.current));
        equal("OK", newinterop.use.UseKt.generic(new GenericBean<String>("O")));
        Bean identity = new Bean();
        equal(true, newinterop.use.UseKt.objectMembers(identity));
        equal(System.identityHashCode(identity), newinterop.use.UseKt.objectHash(identity));
        equal("nullK", newinterop.use.UseKt.platformConcat());
        equal("bean:" + identity.toString(), newinterop.use.UseKt.concatObject(identity));
        equal(0, newinterop.use.UseKt.emptySum());
        equal("hidden:visible:6", newinterop.use.UseKt.hiddenParent(new Facade()));
        if (newinterop.use.UseKt.nullWrite(new Bean()) != null) throw new AssertionError("platform property null lost");
        System.out.println("OK");
    }
}
