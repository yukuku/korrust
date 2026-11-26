package composetypeprobe;
import androidx.compose.runtime.*;
import java.lang.reflect.*;
import kotlin.Unit;
import kotlin.jvm.functions.Function2;
import composeprobe.Registry;
@SuppressWarnings({"unchecked", "rawtypes"})
public class Calls {
    static int hits;
    static Composer expected;
    static final class State implements InvocationHandler {
        Composer composer;
        Object scope;
        int starts, ends;
        State() {
            scope = Proxy.newProxyInstance(getClass().getClassLoader(), new Class[]{RecomposeScope.class,ScopeUpdateScope.class}, (p,m,a)-> {
                if (m.getName().equals("updateScope") || m.getName().equals("invalidate")) return null;
                if (m.getName().equals("equals")) return p == a[0];
                if (m.getName().equals("hashCode")) return System.identityHashCode(p);
                if (m.getName().equals("toString")) return "scope";
                throw new AssertionError(m);
            });
            composer = (Composer)Proxy.newProxyInstance(getClass().getClassLoader(), new Class[]{Composer.class}, this);
        }
        public Object invoke(Object p, Method m, Object[] a) {
            switch (m.getName()) {
                case "startRestartGroup": starts++; return composer;
                case "endRestartGroup": ends++; return scope;
                case "getRecomposeScope": return scope;
                case "recordUsed": return null;
                case "changed": case "changedInstance": return false;
                case "shouldExecute": return true;
                case "getSkipping": case "getDefaultsInvalid": return false;
                case "equals": return p == a[0];
                case "hashCode": return System.identityHashCode(p);
                case "toString": return "composer";
                default: throw new AssertionError(m);
            }
        }
    }
    public static void main(String[] args) {
        Function2<Composer,Integer,Unit> callback = (composer, changed) -> {
            if (composer != expected || changed != 0) throw new AssertionError("hidden callback arguments");
            hits++; return Unit.INSTANCE;
        };
        if (CallbacksKt.identity(callback) != callback || CallbacksKt.qualified(callback) != callback || CallbacksKt.parentheses(callback) != callback) throw new AssertionError("identity");
        Registry registry = new Registry();
        CallbacksKt.forward(registry, callback);
        if (registry.getContent() != callback) throw new AssertionError("typed forward");
        Function2[] callbacks = {CallbacksKt.wrapped(callback), CallbacksKt.explicitInvoke(callback), CallbacksKt.saved(callback)};
        for (Function2 function : callbacks) {
            State state = new State(); expected = state.composer;
            if (function.invoke(expected, 1) != Unit.INSTANCE) throw new AssertionError("Unit result");
            if (state.starts == 0 || state.starts != state.ends) throw new AssertionError("group balance");
        }
        Function2 fromConsumer = composetypeconsumer.UseKt.fromConsumer(registry, callback);
        State consumerState = new State(); expected = consumerState.composer;
        if (fromConsumer.invoke(expected, 1) != Unit.INSTANCE) throw new AssertionError("consumer Unit");
        if (consumerState.starts == 0 || consumerState.starts != consumerState.ends || registry.getContent() != callback) throw new AssertionError("consumer protocol");
        if (hits != 4) throw new AssertionError("callback count");
        System.out.println("source-composable-types-ok");
    }
}
