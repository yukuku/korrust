import kotlin.Metadata;
import kotlin.Pair;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmNameResolver;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;
public class MetadataDump {
 public static void main(String[] args) throws Exception {
  for (String owner : args) {
   Metadata metadata = Class.forName(owner).getAnnotation(Metadata.class);
   Pair<JvmNameResolver, ProtoBuf.Class> decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
   System.out.println(owner+" "+decoded.getSecond().getFlags());
   for (ProtoBuf.Function f : decoded.getSecond().getFunctionList()) System.out.println(decoded.getFirst().getString(f.getName())+" "+f.getFlags());
  }
 }
}
