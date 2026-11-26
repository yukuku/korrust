package composeprobe;
import androidx.compose.runtime.*;
import java.lang.reflect.*;
import java.util.*;
import kotlin.Unit;
import kotlin.jvm.functions.Function2;
public class MemoProtocol {
    static final class Memory implements InvocationHandler {
        final ProtocolCheck.State base;
        final List<Object> slots=new ArrayList<>(), inputs=new ArrayList<>(), functions=new ArrayList<>();
        int cursor,inputCursor,functionCursor,updates,sourceStarts,sourceEnds,expectedFlags;
        final List<String> comparisons=new ArrayList<>();
        final List<Class<?>> descriptors=new ArrayList<>();
        final List<kotlin.jvm.functions.Function0> effects=new ArrayList<>();
        Memory() { this(false); }
        Memory(boolean skipping) {
            base=new ProtocolCheck.State(false,skipping);
            base.composer=(Composer)Proxy.newProxyInstance(getClass().getClassLoader(),new Class[]{Composer.class},this);
        }
        void begin() { cursor=inputCursor=functionCursor=updates=0; comparisons.clear(); descriptors.clear(); }
        public Object invoke(Object p,Method m,Object[] a) {
            switch(m.getName()) {
                case "changed": case "changedInstance":
                    if (a[0] instanceof kotlin.Function) {
                        Object prior=functionCursor<functions.size()?functions.get(functionCursor):Composer.Companion.getEmpty();
                        if(functionCursor==functions.size()) functions.add(a[0]); else functions.set(functionCursor,a[0]);
                        functionCursor++;
                        return prior!=a[0];
                    }
                    boolean identity=m.getName().equals("changedInstance");
                    comparisons.add(identity?"identity":"value");
                    descriptors.add(m.getParameterTypes()[0]);
                    Object old=inputCursor<inputs.size()?inputs.get(inputCursor):Composer.Companion.getEmpty();
                    if(inputCursor==inputs.size()) inputs.add(a[0]); else inputs.set(inputCursor,a[0]);
                    inputCursor++;
                    return identity?old!=a[0]:!Objects.equals(old,a[0]);
                case "sourceInformationMarkerStart": sourceStarts++; return null;
                case "sourceInformationMarkerEnd": sourceEnds++; return null;
                case "recordSideEffect": effects.add((kotlin.jvm.functions.Function0)a[0]); return null;
                case "rememberedValue": return cursor<slots.size()?slots.get(cursor++):empty();
                case "updateRememberedValue":
                    updates++;
                    if(cursor-1==slots.size()) slots.add(a[0]); else slots.set(cursor-1,a[0]);
                    return null;
                default: return base.invoke(p,m,a);
            }
        }
        Object empty() { cursor++; return Composer.Companion.getEmpty(); }
    }
    static final class Host implements MemoHost {
        int value;
        public void hit(int n) { value+=n; }
        public boolean equals(Object other) { return other instanceof Host; }
        public int hashCode() { return 1; }
    }
    static void invoke(Registry registry, Memory memory) {
        memory.begin(); Capture.entries.clear();
        if(((Function2)registry.getContent()).invoke(memory.base.composer,0)!=Unit.INSTANCE) throw new AssertionError();
        if (!Capture.entries.isEmpty()) {
            Capture.Entry entry=Capture.entries.get(Capture.entries.size()-1);
            if(entry.name.equals("callbacks") && (!Arrays.equals(entry.words,new int[]{memory.expectedFlags}) || entry.composer!=memory.base.composer)) throw new AssertionError("callback hidden flags");
        }
    }
    static Object[] callbacks() { return new Object[]{MemoEvents.INSTANCE.getFirst(),MemoEvents.INSTANCE.getSecond(),MemoEvents.INSTANCE.getThird()}; }
    static void same(Object[] left,Object[] right,boolean expected) {
        for(int i=0;i<3;i++) if((left[i]==right[i])!=expected) throw new AssertionError("callback identity "+i);
    }
    public static void main(String[] args) {
        Registry registry=new Registry(); Memory memory=new Memory(); Host one=new Host(),two=new Host();
        ConsumerKt.memoHost(registry,one); invoke(registry,memory);
        Object[] initial=callbacks();
        if(memory.updates!=3 || !memory.comparisons.equals(Arrays.asList("identity","identity","identity"))) throw new AssertionError("initial comparisons "+memory.comparisons);
        MemoEvents.INSTANCE.getFirst().invoke(7); MemoEvents.INSTANCE.getSecond().invoke(); MemoEvents.INSTANCE.getThird().invoke();
        if(one.value!=31) throw new AssertionError("callback bodies");
        invoke(registry,memory); same(initial,callbacks(),true); if(memory.updates!=0) throw new AssertionError("unchanged updates");
        ConsumerKt.memoHost(registry,two); invoke(registry,memory); same(initial,callbacks(),false);
        if(memory.updates!=3) throw new AssertionError("identity invalidation");
        MemoEvents.INSTANCE.getFirst().invoke(5); if(one.value!=31 || two.value!=5) throw new AssertionError("new captured receiver");
        ConsumerKt.memoHost(registry,null); invoke(registry,memory); MemoEvents.INSTANCE.getFirst().invoke(7);
        Object[] nulled=callbacks(); invoke(registry,memory); same(nulled,callbacks(),true);
        memory=new Memory(); memory.expectedFlags=432; ConsumerKt.memoStable(registry,17,new String("abc")); invoke(registry,memory);
        Object[] stable=callbacks(); if(memory.updates!=3 || !memory.comparisons.equals(Arrays.asList("value","value"))) throw new AssertionError("stable comparisons "+memory.comparisons);
        Events.INSTANCE.setValue(0); MemoEvents.INSTANCE.getFirst().invoke(5); MemoEvents.INSTANCE.getThird().invoke();
        if(Events.INSTANCE.getValue()!=62) throw new AssertionError("stable bodies "+Events.INSTANCE.getValue());
        ConsumerKt.memoStable(registry,17,new String("abc")); invoke(registry,memory); same(stable,callbacks(),true);
        ConsumerKt.memoStable(registry,18,null); invoke(registry,memory);
        Object[] changed=callbacks(); if(changed[0]==stable[0] || changed[1]!=stable[1] || changed[2]!=stable[2] || memory.updates!=1) throw new AssertionError("individual cache slots");
        memory=new Memory(); memory.expectedFlags=390; ConsumerKt.memoWide(registry,1L,2.0,3.0f,true,'C',(byte)4,(short)5); invoke(registry,memory);
        Object[] wide=callbacks();
        Set<Class<?>> expected=new HashSet<>(Arrays.asList(long.class,double.class,float.class,boolean.class,char.class,byte.class,short.class));
        if(memory.updates!=3 || memory.descriptors.size()!=7 || !new HashSet<>(memory.descriptors).equals(expected)) throw new AssertionError("primitive overloads "+memory.descriptors);
        Events.INSTANCE.setValue(0); MemoEvents.INSTANCE.getSecond().invoke();
        if(Events.INSTANCE.getValue()!=83) throw new AssertionError("wide bodies "+Events.INSTANCE.getValue());
        invoke(registry,memory); same(wide,callbacks(),true);
        ConsumerKt.memoWide(registry,1L,2.0,3.0f,true,'C',(byte)4,(short)6); invoke(registry,memory);
        if(memory.updates!=1 || memory.descriptors.size()!=7 || callbacks()[1]==wide[1]) throw new AssertionError("all captures evaluated");
        memory=new Memory(true); ConsumerKt.memoHost(registry,one); invoke(registry,memory);
        Object[] skipped=callbacks(); memory.begin();
        ((Function2)registry.getContent()).invoke(memory.base.composer,2);
        same(skipped,callbacks(),true);
        if(memory.cursor!=0 || memory.inputCursor!=0 || memory.updates!=0 || memory.base.skips!=1) throw new AssertionError("skipped cache protocol");
        memory=new Memory(); ConsumerKt.memoEffect(registry); Events.INSTANCE.setValue(0); invoke(registry,memory);
        if(memory.effects.size()!=1 || Events.INSTANCE.getValue()!=0 || memory.updates!=1 || memory.sourceStarts!=1 || memory.sourceEnds!=1) throw new AssertionError("deferred effect");
        memory.effects.remove(0).invoke(); if(Events.INSTANCE.getValue()!=53) throw new AssertionError("effect body");
        memory=new Memory(); ConsumerKt.binaryUnstableResult(registry); Events.INSTANCE.setValue(0); invoke(registry,memory);
        if(Capture.entries.size()!=1 || !Capture.entries.get(0).name.equals("consumePayload") || !Arrays.equals(Capture.entries.get(0).words,new int[]{8}) || Events.INSTANCE.getValue()!=9 || !memory.comparisons.equals(Arrays.asList("identity"))) throw new AssertionError("unstable result flags "+Arrays.toString(Capture.entries.get(0).words)+" effects="+Events.INSTANCE.getValue()+" comparisons="+memory.comparisons);
        System.out.println("memo-protocol-ok");
    }
}
