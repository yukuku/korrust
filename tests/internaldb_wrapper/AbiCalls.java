package wrapperproof;
import kotlin.Unit;
import yuku.alkitab.base.storage.InternalDb;
import yuku.alkitab.base.storage.InternalDbTxWrapper;
public final class AbiCalls {
    public static void run(InternalDb internal) {
        InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
            handle.commit();
            return Unit.INSTANCE;
        });
    }
}
