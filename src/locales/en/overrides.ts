export default {
  title: 'Directory Overrides',
  description: 'Specify toolchain versions for different directories',
  action: {
    addOverride: 'Add Override',
  },
  placeholder: {
    dirPath: 'Directory path...',
    selectToolchain: 'Select toolchain',
  },
  status: {
    noOverrides: 'No directory overrides configured.',
  },
  message: {
    setSuccess: 'Set override for {dir} to {toolchain}',
    setError: 'Error: {error}',
    removeSuccess: 'Removed override for {path}',
    removeError: 'Error: {error}',
  },
}
