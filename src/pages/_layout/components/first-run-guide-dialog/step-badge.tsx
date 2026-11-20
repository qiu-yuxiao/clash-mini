import { Box } from '@mui/material'
import type React from 'react'

interface FirstRunStepBadgeProps {
  currentStep: 1 | 2
  isRetro3D: boolean
  secondaryText: string
  readyColor: string
}

export const FirstRunStepBadge: React.FC<FirstRunStepBadgeProps> = ({
  currentStep,
  isRetro3D,
  secondaryText,
  readyColor,
}) => (
  <Box
    sx={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: 0.75,
      px: 1,
      py: 0.2,
      borderRadius: '999px',
      fontSize: '10.5px',
      fontWeight: 700,
      letterSpacing: '0.6px',
      bgcolor: isRetro3D
        ? 'rgba(44, 31, 3, 0.12)'
        : 'rgba(128, 128, 128, 0.14)',
      color: secondaryText,
      border: '1px solid',
      borderColor: isRetro3D
        ? 'rgba(44, 31, 3, 0.22)'
        : 'rgba(128, 128, 128, 0.24)',
    }}
  >
    <Box
      sx={{
        width: 6,
        height: 6,
        borderRadius: '50%',
        bgcolor: currentStep === 1 ? 'primary.main' : readyColor,
      }}
    />
    <span>STEP {currentStep} / 2</span>
  </Box>
)
