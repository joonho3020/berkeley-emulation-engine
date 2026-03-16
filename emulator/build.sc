// import Mill dependency
import mill._
import mill.define.Sources
import mill.modules.Util
import mill.scalalib.TestModule.ScalaTest
import scalalib._
// support BSP
import mill.bsp._

/** Local rocket-chip checkout path */
def rocketRoot = os.Path("/workspace/rocket-chip")

/** Common settings */
trait CommonScala extends ScalaModule {
  def scalaVersion = "2.13.12"
  def scalacOptions = Seq(
    "-language:reflectiveCalls",
    "-deprecation",
    "-feature",
    "-Xcheckinit",
    "-Ymacro-annotations",
  )


  override def ivyDeps = Agg(
    ivy"org.chipsalliance::chisel:6.0.0"
  )

  override def scalacPluginIvyDeps = Agg(
    ivy"org.chipsalliance:::chisel-plugin:6.0.0"
  )
}
// /** ---- Local rocket-chip submodules (from rocket-chip/dependencies/*) ----
//   * These folders exist in your local clone. We compile them as source modules
//   * so we don't rely on remote SNAPSHOT artifacts.
//   *
//   * Path assumptions (based on typical rocket-chip layout):
//   *   rocket-chip/dependencies/cde/cde/src/...
//   *   rocket-chip/dependencies/diplomacy/diplomacy/src/...
//   *   rocket-chip/dependencies/hardfloat/hardfloat/src/...
//   *
//   * If your folder names differ slightly, adjust millSourcePath accordingly.
//   */
object cde extends CommonScala {
  def millSourcePath = rocketRoot / "dependencies" / "cde" / "cde"
}

object diplomacy extends CommonScala {
  def millSourcePath = rocketRoot / "dependencies" / "diplomacy" / "diplomacy"
  override def moduleDeps = Seq(cde)
}

object hardfloat extends CommonScala {
  def millSourcePath = rocketRoot / "dependencies" / "hardfloat" / "hardfloat"
}

/** ---- Local rocket-chip main module ---- */
object rocketchip extends CommonScala {
  def millSourcePath = rocketRoot

  // Ensure rocket-chip sees its local submodules
  override def moduleDeps = Seq(cde, diplomacy, hardfloat)

  // rocket-chip needs extra libs: mainargs + json4s-jackson
  override def ivyDeps = super.ivyDeps() ++ Agg(
    ivy"com.lihaoyi::mainargs:0.7.6",
    ivy"org.json4s::json4s-jackson:4.0.7"
  )
}


/** ---- Your emulator module ---- */
object emulator extends CommonScala {
  def millSourcePath = os.pwd

  // Make emulator depend on the local rocket-chip build outputs
  override def moduleDeps = Seq(rocketchip)

  override def ivyDeps = Agg(
    ivy"org.chipsalliance::chisel:6.0.0",
    ivy"com.lihaoyi::pprint:0.9.0"
  )

  override def scalacPluginIvyDeps = Agg(
    ivy"org.chipsalliance:::chisel-plugin:6.0.0",
  )

  // Keep these repos (harmless), but no longer required for rocketchip SNAPSHOT
  val sonatypeReleases = Seq(
    coursier.maven.MavenRepository("https://oss.sonatype.org/content/repositories/snapshots/"),
    coursier.maven.MavenRepository("https://oss.sonatype.org/content/repositories/releases/"),
  )
   val extraRepos = Seq(
    coursier.maven.MavenRepository("https://repo.maven.apache.org/maven2"), 
    coursier.maven.MavenRepository("https://maven.aliyun.com/repository/central"),
    coursier.maven.MavenRepository("https://mirrors.cloud.tencent.com/nexus/repository/maven-public/")
  )

  def repositoriesTask = T.task {
    super.repositoriesTask() ++ extraRepos ++ sonatypeReleases
  }
}