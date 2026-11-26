package composeprobe;
import kotlin.jvm.functions.Function2;
import java.util.Arrays;

@SuppressWarnings({"unchecked", "rawtypes"})
public class BinaryProtocol {
    static void check(ProtocolCheck.Trace trace) {
        Registry registry=new Registry();
        for (int mode=0;mode<5;mode++) {
            switch(mode) {
                case 0: ConsumerKt.binaryCaptured(registry,7); break;
                case 1: ConsumerKt.binaryStable(registry); break;
                case 2: ConsumerKt.binaryDefaults(registry); break;
                case 3: ConsumerKt.binaryOrdered(registry); break;
                default: ConsumerKt.binaryMany(registry);
            }
            Function2 callback=(Function2)registry.getContent();
            for(boolean changed:new boolean[]{false,true}) {
                trace.reset(); Events.INSTANCE.setValue(0); BinaryEvents.INSTANCE.setOrder("");
                Capture.entries.clear();
                ProtocolCheck.State state=new ProtocolCheck.State(changed,false);
                callback.invoke(state.composer,0);
                int word=0; for(int i=0;i<10;i++) word|=6<<(3*i);
                int[] words=mode==0?new int[]{48,0}:mode==1?new int[]{54,0}:mode==2?new int[]{0,3}:mode==3?new int[]{0,0}:new int[]{word,6};
                if(Capture.entries.size()!=1 || Capture.entries.get(0).composer!=state.composer || !Arrays.equals(Capture.entries.get(0).words,words)) throw new AssertionError("hidden call arguments mode="+mode+" actual="+Arrays.toString(Capture.entries.get(0).words));
                int expected=mode==0?10:mode==1?12:mode==2?7:mode==3?8:55;
                if(Events.INSTANCE.getValue()!=expected || state.starts!=2 || state.ends!=2 || state.updates!=2) throw new AssertionError("binary compose effects/groups mode="+mode);
                if(trace.starts!=2 || trace.ends!=2) throw new AssertionError("binary compose traces");
                if(mode==3 && !"qn".equals(BinaryEvents.INSTANCE.getOrder())) throw new AssertionError("named argument order");
                Events.INSTANCE.setValue(0);
                ProtocolCheck.State restarted=new ProtocolCheck.State(false,true);
                state.restarts.get(0).invoke(restarted.composer,1);
                if(Events.INSTANCE.getValue()!=expected || restarted.starts!=1 || restarted.ends!=1 || restarted.force!=1) throw new AssertionError("callee restart mode="+mode);
            }
        }
    }
}
