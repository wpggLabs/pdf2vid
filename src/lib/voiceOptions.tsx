import type { Project } from "../types";

/**
 * Maps a Project's voiceProvider to a list of available voice <option>
 * elements. Centralized so the UI and any future preview/test code
 * share the same source of truth.
 */
export function voiceOptionsFor(project: Project): React.ReactElement[] {
  if (project.voiceProvider === "edge") {
    return [
      <option key="en-US-AriaNeural">Aria · English (US)</option>,
      <option key="en-US-JennyNeural">Jenny · English (US)</option>,
      <option key="en-US-GuyNeural">Guy · English (US)</option>,
      <option key="es-ES-ElviraNeural">Elvira · Spanish</option>,
      <option key="fr-FR-DeniseNeural">Denise · French</option>,
      <option key="de-DE-KatjaNeural">Katja · German</option>,
      <option key="hi-IN-SwaraNeural">Swara · Hindi</option>,
      <option key="ja-JP-NanamiNeural">Nanami · Japanese</option>,
      <option key="ko-KR-SunHiNeural">SunHi · Korean</option>,
      <option key="zh-CN-XiaoxiaoNeural">Xiaoxiao · Chinese</option>,
      <option key="ar-EG-SalmaNeural">Salma · Arabic</option>,
    ];
  }
  if (project.voiceProvider === "piper") {
    return [
      <option key="piper-amy">Amy · English (US)</option>,
      <option key="piper-ryan">Ryan · English (US)</option>,
    ];
  }
  if (project.voiceProvider === "elevenlabs") {
    return [
      <option key="eleven-rachel">Rachel · ElevenLabs</option>,
      <option key="eleven-domi">Domi · ElevenLabs</option>,
    ];
  }
  if (project.voiceProvider === "openai") {
    return [
      <option key="openai-alloy">Alloy · OpenAI</option>,
      <option key="openai-shimmer">Shimmer · OpenAI</option>,
      <option key="openai-onyx">Onyx · OpenAI</option>,
    ];
  }
  return [<option key="default">Default voice</option>];
}