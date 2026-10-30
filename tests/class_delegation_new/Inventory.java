import java.nio.file.Files;
import java.nio.file.Path;
import com.intellij.openapi.util.Disposer;
import com.intellij.psi.util.PsiTreeUtil;
import org.jetbrains.kotlin.cli.jvm.compiler.KotlinCoreEnvironment;
import org.jetbrains.kotlin.cli.jvm.compiler.EnvironmentConfigFiles;
import org.jetbrains.kotlin.config.CompilerConfiguration;
import org.jetbrains.kotlin.psi.KtPsiFactory;
import org.jetbrains.kotlin.psi.KtDelegatedSuperTypeEntry;
public class Inventory {
    public static void main(String[] args) throws Exception {
        var disposable=Disposer.newDisposable();
        try {
            var environment=KotlinCoreEnvironment.createForProduction(disposable,new CompilerConfiguration(),EnvironmentConfigFiles.JVM_CONFIG_FILES);
            var factory=new KtPsiFactory(environment.getProject(),false);
            int files=0,hits=0;
            try(var paths=Files.walk(Path.of(args[0]))) {
                for(Path path:paths.filter(p->p.toString().endsWith(".kt")).sorted().toList()) {
                    var file=factory.createFile(path.getFileName().toString(),Files.readString(path));files++;
                    for(var entry:PsiTreeUtil.collectElementsOfType(file,KtDelegatedSuperTypeEntry.class)) {
                        hits++;System.out.println(path+":"+entry.getTextOffset()+":"+entry.getText().replace("\n"," "));
                    }
                }
            }
            System.out.println("FILES="+files+" HEADER_DELEGATIONS="+hits);
        } finally {Disposer.dispose(disposable);}
    }
}
