package annotation17check;

import com.google.samples.apps.nowinandroid.core.common.network.Dispatcher;
import com.google.samples.apps.nowinandroid.core.common.network.NiaDispatchers;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import javax.inject.Qualifier;

@Dispatcher(niaDispatcher = NiaDispatchers.IO)
final class NiaUse {}

public final class NiaCheck {
    public static void main(String[] args) {
        if (NiaUse.class.getAnnotation(Dispatcher.class).niaDispatcher() != NiaDispatchers.IO) throw new AssertionError("enum annotation");
        if (Dispatcher.class.getAnnotation(Qualifier.class) == null) throw new AssertionError("qualifier");
        if (Dispatcher.class.getAnnotation(Retention.class).value() != RetentionPolicy.RUNTIME) throw new AssertionError("retention");
        if (!Dispatcher.class.isAnnotation()) throw new AssertionError("class flags");
        System.out.println("OK:4");
    }
}
