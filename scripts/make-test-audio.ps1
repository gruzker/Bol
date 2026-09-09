$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$taskAudioDirectory = Join-Path $PSScriptRoot '..\artifacts\live-audio'
New-Item -ItemType Directory -Force -Path $taskAudioDirectory | Out-Null
$taskSynth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$taskSynth.Rate = 1
$taskFormat = [System.Speech.AudioFormat.SpeechAudioFormatInfo]::new(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$taskTexts = @(
  'The meeting starts at five. Please bring your notes.',
  'Tomorrow we will review the design, update the project schedule, and send a short summary to the team. Please bring your notes to the meeting. The deadline is Friday at five.',
  'Today we are testing a voice dictation application. The goal is to turn spoken ideas into useful written sentences. First, we will review the design and check the project schedule. Then we will write an email to the team with the latest updates. The meeting starts tomorrow at five, and everyone should bring their notes. We need to make sure that names, numbers, and punctuation are handled correctly. Good dictation should work with short messages and longer paragraphs. It should also let the speaker pause naturally between thoughts. After this test, we will compare the transcript with the original recording and measure how long the result takes to arrive. Please keep the final summary clear, accurate, and easy to read. Thank you for helping us check the application.'
)
try {
  for ($taskIndex = 0; $taskIndex -lt $taskTexts.Count; $taskIndex++) {
    $taskName = @('short', 'medium', 'long')[$taskIndex]
    $taskSynth.SetOutputToWaveFile((Join-Path $taskAudioDirectory "$taskName.wav"), $taskFormat)
    $taskSynth.Speak($taskTexts[$taskIndex])
    $taskSynth.SetOutputToNull()
    [System.IO.File]::WriteAllText((Join-Path $taskAudioDirectory "$taskName.txt"), $taskTexts[$taskIndex])
  }
} finally { $taskSynth.Dispose() }
Write-Output 'Generated three local English speech fixtures.'
