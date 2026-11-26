package composeprobe;

import androidx.compose.runtime.*;
import java.lang.reflect.*;
import java.util.*;
import kotlin.Unit;
import kotlin.jvm.functions.Function2;

@SuppressWarnings({"unchecked", "rawtypes"})
public class ProtocolCheck {
    static final class Trace implements CompositionTracer {
        int checks, starts, ends, key, dirty, dirty2;
        boolean enabled=true;
        String info;
        public boolean isTraceInProgress() { checks++; return enabled; }
        public void traceEventStart(int key, int dirty, int dirty2, String info) {
            starts++; this.key=key; this.dirty=dirty; this.dirty2=dirty2; this.info=info;
        }
        public void traceEventEnd() { ends++; }
        void reset() { checks=starts=ends=0; }
    }
    static final class State implements InvocationHandler {
        final boolean changed, skipping;
        int starts, ends, updates, skips, reads, records, changes, groupKey;
        boolean parameterChanged;
        int force;
        Function2 restart;
        final Object scope;
        Composer composer;
        State(boolean changed, boolean skipping) {
            this.changed=changed; this.skipping=skipping;
            scope=Proxy.newProxyInstance(getClass().getClassLoader(), new Class[]{RecomposeScope.class, ScopeUpdateScope.class}, (p,m,a)-> {
                if (m.getName().equals("updateScope")) { updates++; restart=(Function2)a[0]; return null; }
                if (m.getName().equals("invalidate")) return null;
                if (m.getName().equals("equals")) return p==a[0];
                if (m.getName().equals("hashCode")) return System.identityHashCode(p);
                if (m.getName().equals("toString")) return "scope";
                throw new AssertionError(m);
            });
            composer=(Composer)Proxy.newProxyInstance(getClass().getClassLoader(),new Class[]{Composer.class},this);
        }
        public Object invoke(Object p, Method m, Object[] a) {
            switch(m.getName()) {
                case "startRestartGroup": starts++; groupKey=(Integer)a[0]; return composer;
                case "endRestartGroup": ends++; return scope;
                case "getRecomposeScope": reads++; return scope;
                case "recordUsed": records++; if (a[0]!=scope) throw new AssertionError("scope parameter"); return null;
                case "changed": changes++; return changed;
                case "shouldExecute": parameterChanged=(Boolean)a[0];force=(Integer)a[1]; return parameterChanged || force!=0 || !skipping;
                case "skipToGroupEnd": skips++; return null;
                case "equals": return p==a[0];
                case "hashCode": return System.identityHashCode(p);
                case "toString": return "composer";
                default: throw new AssertionError(m);
            }
        }
    }
    public static void main(String[] args) throws Exception {
        Trace trace=new Trace();
        ComposerKt.class.getDeclaredMethod("access$setCompositionTracer$p", CompositionTracer.class).invoke(null,trace);
        Registry registry=new Registry();
        ConsumerKt.stateless(registry);
        Function2 constant=(Function2)registry.getContent();
        ConsumerKt.stateless(registry);
        if (constant!=registry.getContent()) throw new AssertionError("stateless cache");
        ConsumerKt.captured(registry,7);
        Function2 captured=(Function2)registry.getContent();
        ConsumerKt.captured(registry,8);
        if (captured==registry.getContent()) throw new AssertionError("captured identity");
        int scenarios=0;
        for (int mode=0;mode<2;mode++) {
            Function2 callback=mode==0?constant:captured;
            int amount=mode==0?1:7;
            Integer key=null;
            for(boolean changed:new boolean[]{false,true}) for(boolean skipping:new boolean[]{false,true}) for(int flags=0;flags<16;flags++) {
                trace.reset(); Events.INSTANCE.setValue(0);
                State s=new State(changed,skipping);
                Object result=callback.invoke(s.composer,flags);
                int dirty=flags | (changed?4:2);
                boolean parameters=(dirty&3)!=2;
                boolean execute=parameters || (dirty&1)!=0 || !skipping;
                if (result!=Unit.INSTANCE || s.starts!=1 || s.ends!=1 || s.updates!=1 || s.changes!=1) throw new AssertionError("restart protocol");
                if (s.parameterChanged!=parameters || s.force!=(dirty&1)) throw new AssertionError("hidden parameters");
                if (s.skips!=(execute?0:1) || Events.INSTANCE.getValue()!=(execute?amount:0)) throw new AssertionError("skip/body");
                if (s.reads!=mode || s.records!=mode) throw new AssertionError("capture tracking");
                if (key!=null && key!=s.groupKey) throw new AssertionError("stable key");
                key=s.groupKey;
                if (trace.checks!=(execute?2:0) || trace.starts!=(execute?1:0) || trace.ends!=(execute?1:0)) throw new AssertionError("trace balance");
                if (execute && (trace.key!=s.groupKey || trace.dirty!=dirty || trace.dirty2!=-1 || !trace.info.contains("Consumer.kt"))) throw new AssertionError("trace parameters");
                State again=new State(false,true);
                int before=Events.INSTANCE.getValue();
                s.restart.invoke(again.composer,1);
                if (Events.INSTANCE.getValue()!=before+amount || again.force!=1 || again.starts!=1 || again.ends!=1) throw new AssertionError("recomposition");
                scenarios++;
            }
        }
        ConsumerKt.throughInterface(block -> registry.accept(block), 11);
        checkBody(registry, 11);
        ConsumerKt.throughTopLevel(registry, 13);
        checkBody(registry, 13);
        ConsumerKt.mutable(registry);
        checkBody(registry, 3);
        checkBody(registry, 4);
        ConsumerKt.wide(registry, "abc", 17L, 19.5);
        checkBody(registry, 39);
        ConsumerKt.wide(registry, null, 17L, 19.5);
        checkBody(registry, 36);
        ConsumerKt.localReturn(registry, 23);
        checkBody(registry, 23);
        trace.enabled=false;
        trace.reset();
        checkBody(registry, 23);
        if (trace.checks!=2 || trace.starts!=0 || trace.ends!=0) throw new AssertionError("disabled tracing");
        trace.enabled=true;
        ConsumerKt.exceptional(registry);
        trace.reset();
        State throwing=new State(false,false);
        try {
            ((Function2)registry.getContent()).invoke(throwing.composer,0);
            throw new AssertionError("expected body exception");
        } catch (IllegalStateException expected) {
            if (!"compose-body".equals(expected.getMessage())) throw expected;
        }
        if (trace.starts!=1 || trace.ends!=0 || throwing.starts!=1 || throwing.ends!=0 || throwing.updates!=0) throw new AssertionError("abrupt body protocol");
        Events.INSTANCE.setValue(0);
        trace.reset();
        ConsumerKt.lookalike();
        if (Events.INSTANCE.getValue()!=31 || trace.checks!=0) throw new AssertionError("annotation namespace");
        ComposerKt.class.getDeclaredMethod("access$setCompositionTracer$p", CompositionTracer.class).invoke(null,new Object[]{null});
        System.out.println("compose-protocol-ok:"+scenarios);
    }
    static void checkBody(Registry registry, int expected) {
        Events.INSTANCE.setValue(0);
        State state=new State(false,false);
        if (((Function2)registry.getContent()).invoke(state.composer,0)!=Unit.INSTANCE || Events.INSTANCE.getValue()!=expected) throw new AssertionError("body/capture semantics");
    }
}
