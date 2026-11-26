package inherited.consumer;
import inherited.library.JavaTypes;
public class Calls {
    public static void main(String[] args) {
        Direct direct = new Direct();
        if (!direct.make().tag().equals("direct") || !direct.bodyOnly().tag().equals("body")) throw new AssertionError();
        if (!direct.token().value.equals("token") || !direct.protectedName().equals("protected") || !direct.localShadow().equals("local")) throw new AssertionError();
        if (!new SourceEnd().make().tag().equals("source") || !new CompiledEnd().make().tag().equals("compiled")) throw new AssertionError();
        if (!new Closest().make().tag().equals("near:closest") || !new HiddenSkipped().make().tag().equals("visible-ancestor")) throw new AssertionError();
        if (!new KotlinEnd().make().getValue().equals("kotlin") || !new KotlinEnd().protectedName().equals("k-protected")) throw new AssertionError();
        if (!new Own().make().getValue().equals("own") || !new OwnOpaque().make().getValue().equals("shadow-opaque")) throw new AssertionError();
        if (!new TypeParameter<String>().identity("generic").equals("generic") || !new FunctionParameter().identity("function").equals("function")) throw new AssertionError();
        if (!new BuiltinShadow().name().equals("member-list") || !DeclarationsKt.ordinaryList().get(0).equals("ordinary")) throw new AssertionError();
        JavaTypes.Base.Result value = new JavaTypes.Base.Result("identity");
        if (!new FunctionParameter().valueShadow(value).equals("identity")) throw new AssertionError();
        if (new inherited.explicit.Derived().identity(value) != value || new inherited.same.Derived().identity(value) != value) throw new AssertionError();
        if (!new inherited.library.PackageEnd().name().equals("package")) throw new AssertionError();
        Object suspended = new SuspendEnd().execute(new kotlin.coroutines.Continuation<JavaTypes.Base.Result>() {
            public kotlin.coroutines.CoroutineContext getContext() { return kotlin.coroutines.EmptyCoroutineContext.INSTANCE; }
            public void resumeWith(Object value) { throw new AssertionError("Immediate path unexpectedly resumed"); }
        });
        if (!((JavaTypes.Base.Result) suspended).tag().equals("suspend")) throw new AssertionError();
        System.out.println("inherited-classifiers-ok");
    }
}
